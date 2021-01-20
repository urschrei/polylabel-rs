use crate::polylabel;
use geo::{GeoFloat, LineString, Point, Polygon};
use libc::{c_double, c_void, size_t};
use std::f64;
use std::slice;

/// Wrapper for a void pointer to a sequence of [`Array`](struct.Array.html)s, and the sequence length. Used for FFI.
///
/// Each sequence entry represents an inner Polygon ring.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct WrapperArray {
    pub data: *const Array,
    pub len: size_t,
}

/// Wrapper for a void pointer to a sequence of 2-element arrays representing points, and the sequence length. Used for FFI.
///
/// Used for the outer Polygon shell. `data` is a `Vec<[c_double; 2]>`.
#[repr(C)]
pub struct Array {
    pub data: *const c_void,
    pub len: size_t,
}

/// FFI struct for returned optimum Polygon label position
#[repr(C)]
pub struct Position {
    pub x_pos: c_double,
    pub y_pos: c_double,
}

// convert a Polylabel result Point into values that can be sent across the FFI boundary
impl<T> From<Point<T>> for Position
where
    T: GeoFloat,
{
    fn from(point: Point<T>) -> Position {
        Position {
            x_pos: point.x().to_f64().unwrap() as c_double,
            y_pos: point.y().to_f64().unwrap() as c_double,
        }
    }
}

fn reconstitute(arr: &Array) -> Vec<[f64; 2]> {
    unsafe { slice::from_raw_parts(arr.data as *mut [f64; 2], arr.len).to_vec() }
}

fn reconstitute2(arr: WrapperArray) -> Vec<Vec<[f64; 2]>> {
    let arrays = unsafe { slice::from_raw_parts(arr.data as *mut Array, arr.len) };
    arrays.iter().map(|x| reconstitute(x)).collect()
}

/// FFI access to the [`polylabel`](fn.polylabel.html) function
///
/// Accepts three arguments:
///
/// - an exterior ring [`Array`](struct.Array.html)
/// - zero or more interior rings [`WrapperArray`](struct.WrapperArray.html)
/// - a tolerance `c_double`.
/// If an error occurs while attempting to calculate the label position, the resulting point coordinates
/// will be NaN, NaN.
#[no_mangle]
pub extern "C" fn polylabel_ffi(
    outer: Array,
    inners: WrapperArray,
    tolerance: c_double,
) -> Position {
    let exterior: LineString<_> = unsafe {
        slice::from_raw_parts(outer.data as *mut [c_double; 2], outer.len)
            .to_vec()
            .into()
    };
    let interior: Vec<Vec<[f64; 2]>> = reconstitute2(inners);
    let ls_int: Vec<LineString<c_double>> = interior.into_iter().map(|vec| vec.into()).collect();
    let poly = Polygon::new(exterior, ls_int);
    polylabel(&poly, &tolerance)
        .unwrap_or_else(|_| Point::new(f64::NAN, f64::NAN))
        .into()
}

#[cfg(test)]
mod tests {
    use crate::ffi::{polylabel_ffi, reconstitute2, Array, WrapperArray};
    use geo::Point;
    use libc::{c_void, size_t};
    use std::mem;

    // Only used for testing
    fn gen_array(v: Vec<[f64; 2]>) -> Array {
        let array = Array {
            data: v.as_ptr() as *const c_void,
            len: v.len() as size_t,
        };
        mem::forget(v);
        array
    }
    // only used for testing
    fn gen_wrapperarray(v: Vec<Vec<[f64; 2]>>) -> WrapperArray {
        let converted: Vec<Array> = v.into_iter().map(|x| gen_array(x)).collect();
        let array2 = WrapperArray {
            data: converted.as_ptr() as *const Array,
            len: converted.len() as size_t,
        };
        mem::forget(converted);
        array2
    }
    #[test]
    fn test_array() {
        let i_a = vec![[0.5, 0.5], [1.0, 1.0], [1.5, 0.5]];
        let i_b = vec![[0.55, 0.55], [0.8, 0.8], [1.2, 0.55]];
        let inners = vec![i_a, i_b];
        let array = gen_wrapperarray(inners);
        let rec_inners = reconstitute2(array);
        assert_eq!(rec_inners[0][2], [1.5, 0.5])
    }
    #[test]
    fn test_ffi() {
        let ext_vec = vec![
            [4.0, 1.0],
            [5.0, 2.0],
            [5.0, 3.0],
            [4.0, 4.0],
            [3.0, 4.0],
            [2.0, 3.0],
            [2.0, 2.0],
            [3.0, 1.0],
            [4.0, 1.0],
        ];
        let int_vec = vec![
            vec![[3.5, 3.5], [4.4, 2.0], [2.6, 2.0], [3.5, 3.5]],
            vec![[4.0, 3.0], [4.0, 3.2], [4.5, 3.2], [4.0, 3.0]],
        ];

        let outer = gen_array(ext_vec);
        let inners = gen_wrapperarray(int_vec);
        let res = polylabel_ffi(outer, inners, 0.1);
        let res_point = Point::new(res.x_pos, res.y_pos);
        assert_eq!(res_point, Point::new(3.125, 2.875));
    }
}
