#[macro_use]
extern crate criterion;
extern crate polylabel;

use criterion::Criterion;
use geo::Polygon;
use polylabel::polylabel;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("Threaded", |bencher| {
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
        bencher.iter(|| {
            polylabel(&poly, &10.0);
        });
    });

    c.bench_function("Large Polygon", |bencher| {
        let points = include!("../data/norway_main.rs");
        let poly = Polygon::new(points.into(), vec![]);
        bencher.iter(|| {
            polylabel(&poly, &1.0);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
