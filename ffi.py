#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
ffi.py

Created by Stephan Hügel on 2016-08-25

This file is part of polylabel-rs.

The MIT License (MIT)

Copyright (c) 2016 Stephan Hügel

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.

"""

import os
from sys import platform
from ctypes import Structure, POINTER, c_void_p, c_size_t, c_double, cast, cdll
import numpy as np
from shapely.geometry import Polygon

file_path = os.path.dirname(__file__)
prefix = {'win32': ''}.get(platform, 'lib')
extension = {'darwin': '.dylib', 'win32': '.dll'}.get(platform, '.so')

lib = cdll.LoadLibrary(os.path.join(file_path, "target/release", prefix + "polylabel" + extension))


class _InnersArray(Structure):
    """
    Convert sequence of float lists to a C-compatible void array
    example:
    [
        [[1.0, 2.0], [3.0, 4.0]],
        [[5.0, 6.0], [7.0, 8.0]]
    ]

    Each sequence is an interior Polygon ring

    """
    _fields_ = [("data", c_void_p),
                ("len", c_size_t)]

    @classmethod
    def from_param(cls, seq):
        """  Allow implicit conversions """
        return seq if isinstance(seq, cls) else cls(seq)


    def __init__(self, seq, data_type = c_double):
        ring_array_type = _FFIArray * len(seq)
        ring_array = ring_array_type()
        for i, arr in enumerate([_FFIArray(s) for s in seq]):
            ring_array[i] = arr
        self.data = cast(ring_array, c_void_p)
        self.len = len(seq)

class _FFIArray(Structure):
    """
    Convert sequence of float lists to a C-compatible void array
    example: [[1.0, 2.0], [3.0, 4.0]]

    This represents a Polygon exterior ring
    """
    _fields_ = [("data", c_void_p),
                ("len", c_size_t)]

    @classmethod
    def from_param(cls, seq):
        """  Allow implicit conversions """
        return seq if isinstance(seq, cls) else cls(seq)

    def __init__(self, seq, data_type = c_double):
        array = np.array(seq, dtype=np.float64)
        self._buffer = array.data
        self.data = cast(
            array.ctypes.data_as(POINTER(c_double)),
            c_void_p
        )
        self.len = len(array)

class _CoordResult(Structure):
    """ Container for returned FFI coordinate data """
    _fields_ = [("x_pos", c_double), ("y_pos", c_double)]


def _unpack_coordresult(res, _func, _args):
    """ return our coordinates in a sensible format (a tuple) """
    return res.x_pos, res.y_pos


_labelpos = lib.polylabel_ffi
_labelpos.argtypes = (_FFIArray, _InnersArray, c_double)
_labelpos.restype = _CoordResult
_labelpos.errcheck = _unpack_coordresult


def label_position(ext, interiors=None, tolerance=1.0):
    """
    Calculate the optimum label position within a Polygon
    You may pass either a Shapely Polygon and a tolerance, or
    an exterior ring (list), a list of interior rings (list), and a tolerance

    Accepts Polygon instances as well as lists and lists of lists
    This is a terrible interface, but y'know, dynamic languages

    """
    if isinstance(ext, Polygon):
        _ext = list(ext.exterior.coords)
        _interiors = [list(ring.coords) for ring in ext.interiors]
        return _labelpos(_ext, _interiors, tolerance)
    else:
        return _labelpos(ext, interiors, tolerance)


if __name__ == "__main__":
    # test that everything's working

    exterior = [[4.0, 1.0], [5.0, 2.0], [5.0, 3.0], [4.0, 4.0], [3.0, 4.0], [2.0, 3.0], [2.0, 2.0], [3.0, 1.0], [4.0, 1.0]]
    interiors = [
                    [[3.5, 3.5], [4.4, 2.0], [2.6, 2.0], [3.5, 3.5]],
                    [[4.0, 3.0], [4.0, 3.2], [4.5, 3.2], [4.0, 3.0]],
                ]

    res = label_position(exterior, interiors=interiors, tolerance=0.1)
    if res != (3.125, 2.875):
        raise ValueError('Polylabel returned an incorrect value: %s, %s' % (res[0], res[1]))
    print("Manual polygon:", res)
    # construct a Polygon
    pol = Polygon(exterior, interiors)
    polres = label_position(pol, tolerance=0.1)
    if polres != (3.125, 2.875):
        raise ValueError('Polylabel returned an incorrect value: %s, %s' % (polres[0], polres[1]))
    print ("Shapely Polygon:", polres)
