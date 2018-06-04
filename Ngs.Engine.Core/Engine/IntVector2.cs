//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System.Runtime.InteropServices;

namespace Ngs.Engine {
    /// <summary>
    /// Represents a vector with two integer values.
    /// </summary>
    [StructLayout (LayoutKind.Sequential)]
    public struct IntVector2 {
        /// <summary>
        /// The X component of the vector.
        /// </summary>
        public int X;

        /// <summary>
        /// The Y component of the vector.
        /// </summary>
        public int Y;

        /// <summary>
        /// Creates a <see cref="IntVector2" /> with the specified component values.
        /// </summary>
        /// <param name="x">The value to assign to the <see cref="X" /> field.</param>
        /// <param name="y">The value to assign to the <see cref="Y" /> field.</param>
        public IntVector2 (int x, int y) {
            X = x;
            Y = y;
        }
    }
}