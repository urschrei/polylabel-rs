extern crate num;
use self::num::Float;

extern crate geo;
use self::geo::{Point, Polygon};
use self::geo::algorithm::boundingbox::BoundingBox;
use self::geo::algorithm::centroid::Centroid;
use self::geo::algorithm::distance::Distance;


use std::f64;
use std::collections::BinaryHeap;

/// A helper struct for `polylabel`
/// We're defining it out here because `#[derive]` doesn't work inside functions
#[derive(PartialOrd, PartialEq)]
struct Cell {
    x: f64, // cell centre x
    y: f64, // cell centre y
    h: f64, // half the cell size

    // pointToPolygonDist(x, y, polygon);
    d: f64, // distance from cell center to polygon
    // this.d + this.h * Math.SQRT2;
    max: f64, // max distance to polygon within a cell
}

impl Ord for Cell {
    fn cmp(&self, other: &Cell) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl Eq for Cell {}

// https://github.com/mapbox/polylabel/blob/master/index.js#L82-L101
fn point_to_polygon_dist<T>(point: &Point<T>, polygon: &Polygon<T>)
    where T: Float
{
    // pass
}

// perpendicular distance from a point to a line
fn point_line_distance<T>(point: &Point<T>, start: &Point<T>, end: &Point<T>) -> T
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
mod tests {
}
