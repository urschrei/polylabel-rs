[![Build Status](https://travis-ci.org/urschrei/polylabel-rs.svg?branch=master)](https://travis-ci.org/urschrei/polylabel-rs)
[![Build status](https://ci.appveyor.com/api/projects/status/byle0botr540kcg3?svg=true)](https://ci.appveyor.com/project/urschrei/polylabel-rs)
[![Coverage Status](https://coveralls.io/repos/github/urschrei/polylabel-rs/badge.svg?branch=master)](https://coveralls.io/github/urschrei/polylabel-rs?branch=master)
[![](https://img.shields.io/crates/v/polylabel.svg)](https://crates.io/crates/polylabel)
# Polylabel-rs
A Rust implementation of the [Polylabel](https://github.com/mapbox/polylabel) algorithm
[![GIF](output.gif)]()
# How to Use
```rust
extern crate polylabel;
use polylabel::polylabel;

extern crate geo;
use geo::{Point, LineString, Polygon};

let coords = vec![
    (0.0, 0.0),
    (4.0, 0.0),
    (4.0, 1.0),
    (1.0, 1.0),
    (1.0, 4.0),
    (0.0, 4.0),
    (0.0, 0.0)
];
let ls = LineString(coords.iter().map(|e| Point::new(e.0, e.1)).collect());
let poly = Polygon(ls, vec![]);
let label_pos = polylabel(&poly, &0.10);
// Point(0.5625, 0.5625)
```

## Documentation
https://urschrei.github.io/polylabel-rs/polylabel/index.html

# FFI
Call `polylabel_ffi` with the following three mandatory arguments:
- [`Array`](https://docs.rs/polylabel/0.1.6/polylabel/struct.Array.html) (a void pointer to an array of two-element `c_double`s, the exterior Polygon ring)
- [`WrapperArray`](https://docs.rs/polylabel/0.1.6/polylabel/struct.WrapperArray.html) (a void pointer to an array of `Array`s, the interior Polygon rings, empty if there are none)
- `tolerance`, a `c_float`

The function returns a struct with two `c_double` fields:
- `x_pos`
- `y_pos`

A Python example is available in [`ffi.py`](ffi.py)

# Binaries
Binary libs for:
- `x86_64` *nix (built using `manylinux1`, thus easy to include in Python 2.7 / 3.4 wheels) and OS X
- `i686` and `x86_64` Windows

are available in [releases](releases).

# License
[MIT](license.txt)
