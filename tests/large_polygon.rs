extern crate polylabel;
use polylabel::polylabel;

extern crate geo;
use self::geo::{Point, Polygon};

#[test]
fn large_polygon() {
    let points = include!("../data/norway_main.rs");
    let points_ls: Vec<_> = points.iter().map(|e| Point::new(e[0], e[1])).collect();
    let poly = Polygon::new(points_ls.into(), vec![]);
    let _ = polylabel(&poly, &1.0);
}
