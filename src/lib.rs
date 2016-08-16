extern crate num;
use self::num::Float;

extern crate geo;
use self::geo::{Point, Polygon};
use self::geo::algorithm::boundingbox::BoundingBox;
use self::geo::algorithm::centroid::Centroid;
use self::geo::algorithm::distance::Distance;
use self::geo::algorithm::contains::Contains;


use std::f64;
use std::collections::BinaryHeap;

/// A helper struct for `polylabel`
/// We're defining it out here because `#[derive]` doesn't work inside functions
#[derive(PartialOrd, PartialEq)]
struct Cell<T>
    where T: Float
{
    x: T, // cell centre x
    y: T, // cell centre y
    h: T, // half the cell size

    // pointToPolygonDist(x, y, polygon);
    d: T, // distance from cell center to polygon
    // this.d + this.h * Math.SQRT2;
    max: T, // max distance to polygon within a cell
}

// Signed distance from a Cell's centroid to a Polygon's outline
// Returned value is negative if the point is outside the polygon's exterior ring
impl<T> Cell<T>
    where T: Float
{
    fn distance(&self, polygon: &Polygon<T>) -> T {
        let ref ls = polygon.0;
        let ref points = ls.0;
        let inside = polygon.contains(&Point::new(self.x, self.y));
        let distance = pld(&Point::new(self.x, self.y),
                           &points[0],
                           &points.last().unwrap());
        match inside {
            true => distance,
            false => -distance,
        }
    }
}

impl<T> Ord for Cell<T>
    where T: Float
{
    fn cmp(&self, other: &Cell<T>) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl<T> Eq for Cell<T> where T: Float {}

// perpendicular distance from a point to a line
fn pld<T>(point: &Point<T>, start: &Point<T>, end: &Point<T>) -> T
    where T: Float
{
    if start == end {
        point.distance(start)
    } else {
        let numerator = ((end.x() - start.x()) * (start.y() - point.y()) -
                         (start.x() - point.x()) * (end.y() - start.y()))
            .abs();
        let denominator = start.distance(end);
        numerator / denominator
    }
}

// https://github.com/mapbox/polylabel/blob/master/index.js#L7-L71
fn polylabel<T>(polygon: Polygon<T>, precision: &T)
    where T: Float
{
    // pass
}

#[cfg(test)]
mod tests {}
