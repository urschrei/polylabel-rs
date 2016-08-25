#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
ffi.py

Created by Stephan Hügel on 2016-08-3

This file is part of rdp.

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
import ipdb

file_path = os.path.dirname(__file__)
prefix = {'win32': ''}.get(platform, 'lib')
extension = {'darwin': '.dylib', 'win32': '.dll'}.get(platform, '.so')

lib = cdll.LoadLibrary(os.path.join(file_path, "target/release", prefix + "polylabel" + extension))

class _FFIArray(Structure):
    """
    Convert sequence of float lists to a C-compatible void array
    example: [[1.0, 2.0], [3.0, 4.0]]

    """
    _fields_ = [("data", c_void_p),
                ("len", c_size_t)]

    @classmethod
    def from_param(cls, seq):
        """  Allow implicit conversions """
        return seq if isinstance(seq, cls) else cls(seq)

    def __init__(self, seq, data_type = c_double):
        self.data = cast(
            np.array(seq, dtype=np.float64).ctypes.data_as(POINTER(data_type)),
            c_void_p
        )
        self.len = len(seq)

class _CoordResult(Structure):
    """ Container for returned FFI coordinate data """
    _fields_ = [("x_pos", c_double), ("y_pos", c_double)]

labelpos = lib.polylabel_ffi
labelpos.argtypes = (_FFIArray, _FFIArray, c_double)
labelpos.restype = _CoordResult


if __name__ == "__main__":
    print(labelpos(
        [[4.0, 1.0],
         [5.0, 2.0],
         [5.0, 3.0],
         [4.0, 4.0],
         [3.0, 4.0],
         [2.0, 3.0],
         [2.0, 2.0],
         [3.0, 1.0],
         [4.0, 1.0]],
        [
            [[3.5, 3.5], [4.4, 2.0], [2.6, 2.0], [3.5, 3.5]],
            [[4.0, 3.0], [4.0, 3.2], [4.5, 3.2], [4.0, 3.0]]
        ],
        0.1)
    )
