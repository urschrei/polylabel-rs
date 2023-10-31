#![doc(
    html_logo_url = "https://cdn.rawgit.com/urschrei/polylabel-rs/7a07336e85572eb5faaf0657c2383d7de5620cd8/ell.svg",
    html_root_url = "https://docs.rs/polylabel-rs/"
)]
//! This crate provides a Rust implementation of the [Polylabel](https://github.com/mapbox/polylabel) algorithm
//! for finding the optimum position of a polygon label.
use geo::{prelude::*, Rect};
use geo::{GeoFloat, Point, Polygon};
use num_traits::FromPrimitive;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::iter::Sum;
use std::ops::{Deref, DerefMut};

pub mod errors;
use errors::PolylabelError;

#[cfg(not(target_arch = "wasm32"))]
mod ffi;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::ffi::{polylabel_ffi, Array, Position, WrapperArray};

/// Represention of a Quadtree node's cells. A node contains four Qcells.
#[derive(Debug)]
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
    fn new(centroid: Point<T>, half_extent: T, polygon: &Polygon<T>) -> Qcell<T> {
        let two = T::one() + T::one();
        let distance = signed_distance(centroid, polygon);
        let max_distance = distance + half_extent * two.sqrt();
        Qcell {
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
    fn cmp(&self, other: &Qcell<T>) -> std::cmp::Ordering {
        self.max_distance.partial_cmp(&other.max_distance).unwrap()
    }
}
impl<T> PartialOrd for Qcell<T>
where
    T: GeoFloat,
{
    fn partial_cmp(&self, other: &Qcell<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Eq for Qcell<T> where T: GeoFloat {}
impl<T> PartialEq for Qcell<T>
where
    T: GeoFloat,
{
    fn eq(&self, other: &Qcell<T>) -> bool
    where
        T: GeoFloat,
    {
        self.max_distance == other.max_distance
    }
}

/// Signed distance from a Qcell's centroid to a Polygon's outline
/// Returned value is negative if the point is outside the polygon's exterior ring
fn signed_distance<T>(point: Point<T>, polygon: &Polygon<T>) -> T
where
    T: GeoFloat,
{
    let inside = polygon.contains(&point);
    // Use LineString distance, because Polygon distance returns 0.0 for inside
    let distance = point.euclidean_distance(polygon.exterior());
    if inside {
        distance
    } else {
        -distance
    }
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
    pub fn new(bbox: Rect<T>, half_extent: T, cell_size: T, polygon: &Polygon<T>) -> Self {
        let two = T::one() + T::one();
        // Priority queue
        let mut cell_queue: BinaryHeap<Qcell<T>> = BinaryHeap::new();
        // Build an initial quadtree node, which covers the Polygon
        let mut x = bbox.min().x;
        let mut y;
        while x < bbox.max().x {
            y = bbox.min().y;
            while y < bbox.max().y {
                let point = Point::new(x + half_extent, y + half_extent);
                let latest_dist = signed_distance(point, polygon);
                cell_queue.push(Qcell {
                    centroid: Point::new(x + half_extent, y + half_extent),
                    half_extent,
                    distance: latest_dist,
                    max_distance: latest_dist + half_extent * two.sqrt(),
                });
                y = y + cell_size;
            }
            x = x + cell_size;
        }
        Self(cell_queue)
    }

    pub fn add_quad(&mut self, cell: &Qcell<T>, half_extent: T, polygon: &Polygon<T>) {
        let new_cells = [
            (-T::one(), -T::one()),
            (T::one(), -T::one()),
            (-T::one(), T::one()),
            (T::one(), T::one()),
        ]
        .map(|(dx, dy)| (dx * half_extent, dy * half_extent))
        .map(|(dx, dy)| Point::new(dx, dy))
        .map(|delta| cell.centroid + delta)
        .map(|centeroid| Qcell::new(centeroid, half_extent, polygon));
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
    let two = T::one() + T::one();
    // Initial best cell values
    let centroid = polygon
        .centroid()
        .ok_or(PolylabelError::CentroidCalculation)?;
    let bbox = polygon
        .bounding_rect()
        .ok_or(PolylabelError::RectCalculation)?;
    let width = bbox.max().x - bbox.min().x;
    let height = bbox.max().y - bbox.min().y;
    let cell_size = width.min(height);
    // Special case for degenerate polygons
    if cell_size == T::zero() {
        return Ok(Point::new(bbox.min().x, bbox.min().y));
    }
    let mut half_extent = cell_size / two;

    let mut best_cell = Qcell::new(centroid, T::zero(), polygon);

    // special case for rectangular polygons
    let bbox_cell = Qcell::new(bbox.centroid(), T::zero(), polygon);

    if bbox_cell.distance > best_cell.distance {
        best_cell = bbox_cell;
    }

    let mut cell_queue = QuadTree::<T>::new(bbox, half_extent, cell_size, polygon);

    // Now try to find better solutions
    while let Some(cell) = cell_queue.pop() {
        // Update the best cell if we find a cell with greater distance
        if cell.distance > best_cell.distance {
            best_cell.centroid = Point::new(cell.centroid.x(), cell.centroid.y());
            best_cell.half_extent = cell.half_extent;
            best_cell.distance = cell.distance;
            best_cell.max_distance = cell.max_distance;
        }
        // Bail out of this iteration if we can't find a better solution
        if cell.max_distance - best_cell.distance <= *tolerance {
            continue;
        }
        // Otherwise, add a new quadtree node and start again
        half_extent = cell.half_extent / two;
        cell_queue.add_quad(&cell, half_extent, polygon);
    }

    // We've exhausted the queue, so return the best solution we've found
    Ok(Point::new(best_cell.centroid.x(), best_cell.centroid.y()))
}

#[cfg(test)]
mod tests {
    use super::{polylabel, Qcell};
    use geo::prelude::*;
    use geo::{Point, Polygon};
    use std::collections::BinaryHeap;
    #[test]
    // polygons are those used in Shapely's tests
    fn test_polylabel() {
        let coords = include!("../tests/fixtures/poly1.rs");
        let poly = Polygon::new(coords.into(), vec![]);
        let res = polylabel(&poly, &10.000).unwrap();
        assert_eq!(res, Point::new(59.35615556364569, 121.83919629746435));
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
        assert_eq!(res, Point::new(-0.45556816445920356, 51.54848888202887));
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
