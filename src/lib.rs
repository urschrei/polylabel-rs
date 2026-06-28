#![doc(
    html_logo_url = "https://cdn.rawgit.com/urschrei/polylabel-rs/7a07336e85572eb5faaf0657c2383d7de5620cd8/ell.svg",
    html_root_url = "https://docs.rs/polylabel-rs/"
)]
//! This crate provides a Rust implementation of the [Polylabel](https://github.com/mapbox/polylabel) algorithm
//! for finding the optimum position of a polygon label.
//!
//! ffi bindings are provided: enable the `ffi` and `headers` features when building the crate.
use geo::{Coord, Line, LineString, Rect, prelude::*};
use geo::{GeoFloat, Point, Polygon};
use num_traits::FromPrimitive;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::iter::Sum;
use std::ops::{Deref, DerefMut};

pub mod errors;
use errors::PolylabelError;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "ffi")]
mod ffi;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "ffi")]
pub use crate::ffi::{Array, Position, WrapperArray, polylabel_ffi};

/// Represention of a Quadtree node's cells. A node contains four Qcells.
#[derive(Debug, Copy, Clone)]
struct Qcell<T>
where
    T: GeoFloat,
{
    // The cell's centroid
    centroid: Point<T>,
    // Half of the parent node's extent
    half_extent: T,
    // Distance from centroid to polygon
    distance: T,
    // Maximum distance to polygon within a cell
    max_distance: T,
}

impl<T> Qcell<T>
where
    T: GeoFloat,
{
    fn new(centroid: Point<T>, half_extent: T, prepared: &PreparedPolygon<T>) -> Self {
        let two = T::one() + T::one();
        let distance = signed_distance(centroid, prepared);
        let max_distance = distance + half_extent * two.sqrt();
        Self {
            centroid,
            half_extent,
            distance,
            max_distance,
        }
    }
}

impl<T> Ord for Qcell<T>
where
    T: GeoFloat,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.max_distance.partial_cmp(&other.max_distance).unwrap()
    }
}
impl<T> PartialOrd for Qcell<T>
where
    T: GeoFloat,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Eq for Qcell<T> where T: GeoFloat {}
impl<T> PartialEq for Qcell<T>
where
    T: GeoFloat,
{
    fn eq(&self, other: &Self) -> bool
    where
        T: GeoFloat,
    {
        self.max_distance == other.max_distance
    }
}

/// Number of consecutive edges grouped under a single bounding box for block-skip
const BLOCK_SIZE: usize = 32;

/// A polygon's rings pre-decomposed into edges, with the edges grouped into
/// fixed-size blocks each carrying a bounding box. Built once per `polylabel`
/// call so the per-cell distance scan can skip whole blocks of edges in O(1).
struct PreparedPolygon<T>
where
    T: GeoFloat,
{
    rings: Vec<PreparedRing<T>>,
}

/// A single ring's edges and the bounding box of each consecutive block of them
struct PreparedRing<T>
where
    T: GeoFloat,
{
    lines: Vec<Line<T>>,
    blocks: Vec<Rect<T>>,
}

impl<T> PreparedPolygon<T>
where
    T: GeoFloat,
{
    fn new(polygon: &Polygon<T>) -> Self {
        let rings = std::iter::once(polygon.exterior())
            .chain(polygon.interiors())
            .map(PreparedRing::new)
            .collect();
        Self { rings }
    }
}

impl<T> PreparedRing<T>
where
    T: GeoFloat,
{
    fn new(ring: &LineString<T>) -> Self {
        let lines: Vec<Line<T>> = ring.lines().collect();
        let blocks = lines.chunks(BLOCK_SIZE).map(block_bbox).collect();
        Self { lines, blocks }
    }
}

/// Bounding box enclosing every endpoint of a block of edges
fn block_bbox<T>(lines: &[Line<T>]) -> Rect<T>
where
    T: GeoFloat,
{
    let mut min_x = T::infinity();
    let mut min_y = T::infinity();
    let mut max_x = T::neg_infinity();
    let mut max_y = T::neg_infinity();
    for line in lines {
        for c in [line.start, line.end] {
            if c.x < min_x {
                min_x = c.x;
            }
            if c.x > max_x {
                max_x = c.x;
            }
            if c.y < min_y {
                min_y = c.y;
            }
            if c.y > max_y {
                max_y = c.y;
            }
        }
    }
    Rect::new(Coord { x: min_x, y: min_y }, Coord { x: max_x, y: max_y })
}

/// Signed distance from a Qcell's centroid to a Polygon's outline
/// Returned value is negative if the point is outside the polygon's exterior ring
///
/// A single pass over the rings accumulates both the even-odd ray-cast parity
/// (inside/outside) and the minimum distance to the outline. Edges are grouped
/// into blocks (see `PreparedPolygon`): a cheap point-to-bounding-box lower
/// bound skips a whole block when it can neither hold a nearer edge nor flip the
/// ray-cast parity. Most blocks of a ring are far from any given cell centre, so
/// this skips the bulk of the edges. The per-segment distance is computed by geo.
fn signed_distance<T>(point: Point<T>, prepared: &PreparedPolygon<T>) -> T
where
    T: GeoFloat,
{
    let x = point.x();
    let y = point.y();
    let mut inside = false;
    let mut min_distance = T::infinity();

    for ring in &prepared.rings {
        for (block, bbox) in ring.blocks.iter().enumerate() {
            let start = block * BLOCK_SIZE;
            let end = (start + BLOCK_SIZE).min(ring.lines.len());
            let bmin = bbox.min();
            let bmax = bbox.max();

            // lower bound on the distance from the point to any edge in this
            // block: a point-to-bounding-box clamp, zero when inside the box
            let dx = if x < bmin.x {
                bmin.x - x
            } else if x > bmax.x {
                x - bmax.x
            } else {
                T::zero()
            };
            let dy = if y < bmin.y {
                bmin.y - y
            } else if y > bmax.y {
                y - bmax.y
            } else {
                T::zero()
            };
            let skip_dist = dx * dx + dy * dy >= min_distance * min_distance;

            // edges here can only flip parity if the bbox straddles y and
            // extends right of x; otherwise no edge crosses the rightward ray
            let skip_cross = y < bmin.y || y >= bmax.y || x > bmax.x;

            if skip_dist && skip_cross {
                continue;
            }

            for line in &ring.lines[start..end] {
                let a = line.start;
                let b = line.end;

                if !skip_cross
                    && ((a.y > y) != (b.y > y))
                    && (x < (b.x - a.x) * (y - a.y) / (b.y - a.y) + a.x)
                {
                    inside = !inside;
                }

                if !skip_dist {
                    min_distance = min_distance.min(Euclidean.distance(&point, line));
                }
            }
        }
    }

    if inside { min_distance } else { -min_distance }
}

struct QuadTree<T>(pub BinaryHeap<Qcell<T>>)
where
    T: GeoFloat;

impl<T> Deref for QuadTree<T>
where
    T: GeoFloat,
{
    type Target = BinaryHeap<Qcell<T>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> DerefMut for QuadTree<T>
where
    T: GeoFloat,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> QuadTree<T>
where
    T: GeoFloat,
{
    pub fn new(bbox: Rect<T>, half_extent: T, prepared: &PreparedPolygon<T>) -> Self {
        let mut cell_queue: BinaryHeap<Qcell<T>> = BinaryHeap::new();

        let two = T::one() + T::one();
        let cell_size = half_extent * two;

        let nx = (bbox.width() / cell_size).ceil().to_usize();
        let ny = (bbox.height() / cell_size).ceil().to_usize();

        if let (Some(nx), Some(ny)) = (nx, ny) {
            let one = T::one();
            let delta_mid = Coord { x: one, y: one } * half_extent;
            let origin = bbox.min();
            let inital_points = (0..nx)
                .flat_map(|x| (0..ny).map(move |y| (x, y)))
                .filter_map(|(x, y)| Some((T::from(x)?, T::from(y)?)))
                .map(|(x, y)| Coord { x, y } * cell_size)
                .map(|delta_cell| origin + delta_cell + delta_mid)
                .map(Point::from)
                .map(|centroid| Qcell::new(centroid, half_extent, prepared));
            cell_queue.extend(inital_points);
        } else {
            // Do nothing, maybe error instead?
        }

        Self(cell_queue)
    }

    pub fn add_quad(&mut self, cell: &Qcell<T>, half_extent: T, prepared: &PreparedPolygon<T>) {
        let new_cells = [
            (-T::one(), -T::one()),
            (T::one(), -T::one()),
            (-T::one(), T::one()),
            (T::one(), T::one()),
        ]
        .map(|(sign_x, sign_y)| (sign_x * half_extent, sign_y * half_extent))
        .map(|(dx, dy)| Point::new(dx, dy))
        .map(|delta| cell.centroid + delta)
        .map(|centroid| Qcell::new(centroid, half_extent, prepared));
        self.extend(new_cells);
    }
}

/// Calculate a Polygon's ideal label position by calculating its ✨pole of inaccessibility✨
///
/// The calculation uses an [iterative grid-based algorithm](https://github.com/mapbox/polylabel#how-the-algorithm-works).
///
/// # Examples
///
/// ```
/// use polylabel::polylabel;
/// extern crate geo;
/// use geo::{Point, LineString, Polygon};
/// use geo::prelude::*;
///
/// // An approximate `L` shape
/// let coords = vec![
///    (0.0, 0.0),
///    (4.0, 0.0),
///    (4.0, 1.0),
///    (1.0, 1.0),
///    (1.0, 4.0),
///    (0.0, 4.0),
///    (0.0, 0.0)];
///
/// let poly = Polygon::new(coords.into(), vec![]);
///
/// // Its centroid lies outside the polygon
/// assert_eq!(poly.centroid().unwrap(), Point::new(1.3571428571428572, 1.3571428571428572));
///
/// let label_position = polylabel(&poly, &0.1).unwrap();
/// // Optimum label position is inside the polygon
/// assert_eq!(label_position, Point::new(0.5625, 0.5625));
/// ```
///
pub fn polylabel<T>(polygon: &Polygon<T>, tolerance: &T) -> Result<Point<T>, PolylabelError>
where
    T: GeoFloat + FromPrimitive + Sum,
{
    // special case for degenerate polygons
    if polygon.signed_area() == T::zero() {
        return Ok(Point::new(T::zero(), T::zero()));
    }

    let bbox = polygon
        .bounding_rect()
        .ok_or(PolylabelError::RectCalculation)?;
    let cell_size = bbox.width().min(bbox.height());
    // Special case for degenerate polygons
    if cell_size == T::zero() {
        return Ok(Point::from(bbox.min()));
    }

    let two = T::one() + T::one();
    let mut half_extent = cell_size / two;

    // decompose the rings into blocks of edges once, up front
    let prepared = PreparedPolygon::new(polygon);

    // initial best guess using centroid
    let centroid = polygon
        .centroid()
        .ok_or(PolylabelError::CentroidCalculation)?;
    let centroid_cell = Qcell::new(centroid, T::zero(), &prepared);

    // special case guess for rectangular polygons
    let bbox_cell = Qcell::new(bbox.centroid(), T::zero(), &prepared);

    // deciding which initial guess was better
    let mut best_cell = if bbox_cell.distance < centroid_cell.distance {
        bbox_cell
    } else {
        centroid_cell
    };

    // setup priority queue
    let mut cell_queue = QuadTree::<T>::new(bbox, half_extent, &prepared);

    // Now try to find better solutions
    while let Some(cell) = cell_queue.pop() {
        // Update the best cell if we find a cell with greater distance
        if cell.distance > best_cell.distance {
            best_cell = Qcell { ..cell };
        }

        // Bail out of this iteration if we can't find a better solution
        if cell.max_distance - best_cell.distance <= *tolerance {
            continue;
        }

        // Otherwise, add a new quadtree node and start again
        half_extent = cell.half_extent / two;
        cell_queue.add_quad(&cell, half_extent, &prepared);
    }

    // We've exhausted the queue, so return the best solution we've found
    Ok(best_cell.centroid)
}

#[cfg(test)]
mod tests {
    use super::{Qcell, polylabel};
    use geo::prelude::*;
    use geo::{LineString, Point, Polygon};
    use std::collections::BinaryHeap;
    #[test]
    // polygons are those used in Shapely's tests
    fn test_polylabel() {
        let coords = include!("../tests/fixtures/poly1.rs");
        let poly = Polygon::new(coords.into(), vec![]);
        let res = polylabel(&poly, &10.000).unwrap();
        assert_eq!(
            res,
            Point::new(59.356_155_563_645_69, 121.839_196_297_464_35)
        );
    }
    #[test]
    // does a concave polygon contain the calculated point?
    // relevant because the centroid lies outside the polygon in this case
    fn test_concave() {
        let coords = include!("../tests/fixtures/poly2.rs");
        let poly = Polygon::new(coords.into(), vec![]);
        let res = polylabel(&poly, &1.0).unwrap();
        assert!(poly.contains(&res));
    }
    #[test]
    fn test_london() {
        let coords = include!("../tests/fixtures/poly3.rs");
        let poly = Polygon::new(coords.into(), vec![]);
        let res = polylabel(&poly, &0.001).unwrap();
        assert_eq!(
            res,
            Point::new(-0.455_568_164_459_203_56, 51.548_488_882_028_87)
        );
    }
    #[test]
    fn polygon_l_test() {
        // an L shape
        let coords = vec![
            (0.0, 0.0),
            (4.0, 0.0),
            (4.0, 1.0),
            (1.0, 1.0),
            (1.0, 4.0),
            (0.0, 4.0),
            (0.0, 0.0),
        ];
        let poly = Polygon::new(coords.into(), vec![]);
        let res = polylabel(&poly, &0.10).unwrap();
        assert_eq!(res, Point::new(0.5625, 0.5625));
    }
    #[test]
    fn degenerate_polygon_test() {
        let a_coords = vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (0.0, 0.0)];
        let a_poly = Polygon::new(a_coords.into(), vec![]);
        let a_res = polylabel(&a_poly, &1.0).unwrap();
        assert_eq!(a_res, Point::new(0.0, 0.0));
    }
    #[test]
    fn degenerate_polygon_test_2() {
        let b_coords = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)];
        let b_poly = Polygon::new(b_coords.into(), vec![]);
        let b_res = polylabel(&b_poly, &1.0).unwrap();
        assert_eq!(b_res, Point::new(0.0, 0.0));
    }
    #[test]
    fn polygon_with_hole_test() {
        let outer = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
        let inner = vec![(60.0, 60.0), (60.0, 80.0), (80.0, 80.0), (80.0, 60.0)];
        let hole_poly = Polygon::new(LineString::from(outer), vec![LineString::from(inner)]);
        let hole_res = polylabel(&hole_poly, &1.0).unwrap();
        assert_eq!(hole_res, Point::new(35.15625, 35.15625));
    }
    #[test]
    // Is our priority queue behaving as it should?
    fn test_queue() {
        let a = Qcell {
            centroid: Point::new(1.0, 2.0),
            half_extent: 3.0,
            distance: 4.0,
            max_distance: 8.0,
        };
        let b = Qcell {
            centroid: Point::new(1.0, 2.0),
            half_extent: 3.0,
            distance: 4.0,
            max_distance: 7.0,
        };
        let c = Qcell {
            centroid: Point::new(1.0, 2.0),
            half_extent: 3.0,
            distance: 4.0,
            max_distance: 9.0,
        };
        let v = vec![a, b, c];
        let mut q = BinaryHeap::from(v);
        assert_eq!(q.pop().unwrap().max_distance, 9.0);
        assert_eq!(q.pop().unwrap().max_distance, 8.0);
        assert_eq!(q.pop().unwrap().max_distance, 7.0);
    }
}
