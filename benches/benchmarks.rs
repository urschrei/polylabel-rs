#![feature(test)]

extern crate test;
use test::Bencher;

use ::geo::{LineString, Point, Polygon};
use polylabel::polylabel;

#[bench]
fn bench_threads(b: &mut Bencher) {
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
    let ls: LineString<_> = coords.into();
    let poly = Polygon::new(ls, vec![]);
    b.iter(|| {
        polylabel(&poly, &10.0);
    });
}

#[bench]
fn large_polygon(b: &mut Bencher) {
    let points = include!("../data/norway_main.rs");
    let points_ls: Vec<_> = points.iter().map(|e| Point::new(e[0], e[1])).collect();
    let poly = Polygon::new(points_ls.into(), vec![]);
    b.iter(|| {
        polylabel(&poly, &1.0);
    });
}
