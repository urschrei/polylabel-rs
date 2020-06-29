#![doc(
    html_logo_url = "https://cdn.rawgit.com/urschrei/polylabel-rs/7a07336e85572eb5faaf0657c2383d7de5620cd8/ell.svg",
    html_root_url = "https://docs.rs/polylabel-rs/"
)]
//! This crate provides a Rust implementation of the [Polylabel](https://github.com/mapbox/polylabel) algorithm
//! for finding the optimum position of a polygon label.
use geo::prelude::*;
use geo::{Point, Polygon};
use num_traits::{Float, FromPrimitive, Signed};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::iter::Sum;

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
    T: Float + Signed,
{
    // The cell's centroid
    centroid: Point<T>,
    // Half of the parent node's extent
    extent: T,
    // Distance from centroid to polygon
    distance: T,
    // Maximum distance to polygon within a cell
    max_distance: T,
}

impl<T> Qcell<T>
where
    T: Float + Signed,
{
    fn new(x: T, y: T, h: T, distance: T, max_distance: T) -> Qcell<T> {
        Qcell {
            centroid: Point::new(x, y),
            extent: h,
            distance,
            max_distance,
        }
    }
}

impl<T> Ord for Qcell<T>
where
    T: Float + Signed,
{
    fn cmp(&self, other: &Qcell<T>) -> std::cmp::Ordering {
        self.max_distance.partial_cmp(&other.max_distance).unwrap()
    }
}
impl<T> PartialOrd for Qcell<T>
where
    T: Float + Signed,
{
    fn partial_cmp(&self, other: &Qcell<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Eq for Qcell<T> where T: Float + Signed {}
impl<T> PartialEq for Qcell<T>
where
    T: Float + Signed,
{
    fn eq(&self, other: &Qcell<T>) -> bool
    where
        T: Float,
    {
        self.max_distance == other.max_distance
    }
}

/// Signed distance from a Qcell's centroid to a Polygon's outline
/// Returned value is negative if the point is outside the polygon's exterior ring
fn signed_distance<T>(x: &T, y: &T, polygon: &Polygon<T>) -> T
where
    T: Float,
{
    let point = Point::new(*x, *y);
    let inside = polygon.contains(&point);
    // Use LineString distance, because Polygon distance returns 0.0 for inside
    let distance = point.euclidean_distance(polygon.exterior());
    if inside {
        distance
    } else {
        -distance
    }
}

/// Add a new Quadtree node made up of four `Qcell`s to the binary heap
fn add_quad<T>(
    mpq: &mut BinaryHeap<Qcell<T>>,
    cell: &Qcell<T>,
    new_height: &T,
    polygon: &Polygon<T>,
) where
    T: Float + Signed,
{
    let two = T::one() + T::one();
    let centroid_x = cell.centroid.x();
    let centroid_y = cell.centroid.y();
    for combo in &[
        (centroid_x - *new_height, centroid_y - *new_height),
        (centroid_x + *new_height, centroid_y - *new_height),
        (centroid_x - *new_height, centroid_y + *new_height),
        (centroid_x + *new_height, centroid_y + *new_height),
    ] {
        let new_dist = signed_distance(&combo.0, &combo.1, polygon);
        mpq.push(Qcell::new(
            combo.0,
            combo.1,
            *new_height,
            new_dist,
            new_dist + *new_height * two.sqrt(),
        ));
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
    T: Float + FromPrimitive + Signed + Sum,
{
    // special case for degenerate polygons
    if polygon.signed_area() == T::zero() {
        return Ok(Point::new(T::zero(), T::zero()));
    }
    let two = T::one() + T::one();
    // Initial best cell values
    let centroid = polygon
        .centroid()
        .ok_or_else(|| PolylabelError::CentroidCalculation)?;
    let bbox = polygon
        .bounding_rect()
        .ok_or_else(|| PolylabelError::RectCalculation)?;
    let width = bbox.max().x - bbox.min().x;
    let height = bbox.max().y - bbox.min().y;
    let cell_size = width.min(height);
    // Special case for degenerate polygons
    if cell_size == T::zero() {
        return Ok(Point::new(bbox.min().x, bbox.min().y));
    }
    let mut h = cell_size / two;
    let distance = signed_distance(&centroid.x(), &centroid.y(), polygon);
    let max_distance = distance + T::zero() * two.sqrt();

    let mut best_cell = Qcell::new(
        centroid.x(),
        centroid.y(),
        T::zero(),
        distance,
        max_distance,
    );

    // special case for rectangular polygons
    let bbox_cell_dist = signed_distance(
        &(bbox.min().x + width / two),
        &(bbox.min().y + height / two),
        polygon,
    );
    let bbox_cell = Qcell {
        centroid: Point::new(bbox.min().x + width / two, bbox.min().y + height / two),
        extent: T::zero(),
        distance: bbox_cell_dist,
        max_distance: bbox_cell_dist + T::zero() * two.sqrt(),
    };

    if bbox_cell.distance > best_cell.distance {
        best_cell = bbox_cell;
    }

    // Priority queue
    let mut cell_queue: BinaryHeap<Qcell<T>> = BinaryHeap::new();
    // Build an initial quadtree node, which covers the Polygon
    let mut x = bbox.min().x;
    let mut y;
    while x < bbox.max().x {
        y = bbox.min().y;
        while y < bbox.max().y {
            let latest_dist = signed_distance(&(x + h), &(y + h), polygon);
            cell_queue.push(Qcell {
                centroid: Point::new(x + h, y + h),
                extent: h,
                distance: latest_dist,
                max_distance: latest_dist + h * two.sqrt(),
            });
            y = y + cell_size;
        }
        x = x + cell_size;
    }
    // Now try to find better solutions
    while !cell_queue.is_empty() {
        let cell = cell_queue.pop().ok_or_else(|| PolylabelError::EmptyQueue)?;
        // Update the best cell if we find a cell with greater distance
        if cell.distance > best_cell.distance {
            best_cell.centroid = Point::new(cell.centroid.x(), cell.centroid.y());
            best_cell.extent = cell.extent;
            best_cell.distance = cell.distance;
            best_cell.max_distance = cell.max_distance;
        }
        // Bail out of this iteration if we can't find a better solution
        if cell.max_distance - best_cell.distance <= *tolerance {
            continue;
        }
        // Otherwise, add a new quadtree node and start again
        h = cell.extent / two;
        add_quad(&mut cell_queue, &cell, &h, polygon);
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
        let coords = include!("poly1.rs");
        let poly = Polygon::new(coords.into(), vec![]);
        let res = polylabel(&poly, &10.000).unwrap();
        assert_eq!(res, Point::new(59.35615556364569, 121.83919629746435));
    }
    #[test]
    // does a concave polygon contain the calculated point?
    // relevant because the centroid lies outside the polygon in this case
    fn test_concave() {
        let coords = include!("poly2.rs");
        let poly = Polygon::new(coords.into(), vec![]);
        let res = polylabel(&poly, &1.0).unwrap();
        assert!(poly.contains(&res));
    }
    #[test]
    fn test_london() {
        let coords = include!("poly3.rs");
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
            extent: 3.0,
            distance: 4.0,
            max_distance: 8.0,
        };
        let b = Qcell {
            centroid: Point::new(1.0, 2.0),
            extent: 3.0,
            distance: 4.0,
            max_distance: 7.0,
        };
        let c = Qcell {
            centroid: Point::new(1.0, 2.0),
            extent: 3.0,
            distance: 4.0,
            max_distance: 9.0,
        };
        let mut v = vec![];
        v.push(a);
        v.push(b);
        v.push(c);
        let mut q = BinaryHeap::from(v);
        assert_eq!(q.pop().unwrap().max_distance, 9.0);
        assert_eq!(q.pop().unwrap().max_distance, 8.0);
        assert_eq!(q.pop().unwrap().max_distance, 7.0);
    }
}
