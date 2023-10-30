/* Generated with cbindgen:0.24.5 */

/* Warning, this file is autogenerated by cbindgen. Don't modify this manually. */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * FFI struct for returned optimum Polygon label position
 */
typedef struct Position {
    double x_pos;
    double y_pos;
} Position;

/**
 * Wrapper for a void pointer to a sequence of 2-element arrays representing points, and the sequence length. Used for FFI.
 *
 * Used for the outer Polygon shell. `data` is a `Vec<[c_double; 2]>`.
 */
typedef struct Array {
    const void *data;
    size_t len;
} Array;

/**
 * Wrapper for a void pointer to a sequence of [`Array`](struct.Array.html)s, and the sequence length. Used for FFI.
 *
 * Each sequence entry represents an inner Polygon ring.
 */
typedef struct WrapperArray {
    const struct Array *data;
    size_t len;
} WrapperArray;

/**
 * FFI access to the [`polylabel`](fn.polylabel.html) function
 *
 * Accepts three arguments:
 *
 * - an exterior ring representing a Polygon shell or closed LineString
 * - zero or more interior rings representing Polygon holes
 * - a tolerance `c_double`.
 * If an error occurs while attempting to calculate the label position, the resulting point coordinates
 * will be `NaN, NaN`.
 */
struct Position polylabel_ffi(struct Array outer,
                              struct WrapperArray inners,
                              double tolerance);
