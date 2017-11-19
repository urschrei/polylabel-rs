#![feature(test)]

extern crate test;
use test::Bencher;

extern crate polylabel;
use polylabel::polylabel;

extern crate geo;
use self::geo::{Point, LineString, Polygon};

#[bench]
fn bench_threads(b: &mut Bencher) {
    // an L shape
    let coords = vec![(0.0, 0.0), (4.0, 0.0), (4.0, 1.0), (1.0, 1.0), (1.0, 4.0), (0.0, 4.0),
                      (0.0, 0.0)];
    let ls = LineString(coords.iter().map(|e| Point::new(e.0, e.1)).collect());
    let poly = Polygon::new(ls, vec![]);
    b.iter(|| { polylabel(&poly, &10.0); });
}

#[bench]
fn large_polygon(b: &mut Bencher) {
    let points = include!("../data/norway_main.rs");
    let points_ls: Vec<_> = points.iter().map(|e| Point::new(e[0], e[1])).collect();
    let poly = Polygon::new(points_ls.into(), vec![]);
    b.iter(|| { polylabel(&poly, &10.0); });
}
