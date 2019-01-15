#![feature(test)]

extern crate test;
use test::Bencher;

use ::geo::{Polygon};
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
    let poly = Polygon::new(coords.into(), vec![]);
    b.iter(|| {
        polylabel(&poly, &10.0);
    });
}

#[bench]
fn large_polygon(b: &mut Bencher) {
    let points = include!("../data/norway_main.rs");
    let poly = Polygon::new(points.into(), vec![]);
    b.iter(|| {
        polylabel(&poly, &1.0);
    });
}
