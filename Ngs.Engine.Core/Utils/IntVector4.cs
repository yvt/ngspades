//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System.Runtime.InteropServices;
namespace Ngs.Utils {
    /// <summary>
    /// Represents a vector with four integer values.
    /// </summary>
    [StructLayout (LayoutKind.Sequential)]
    public struct IntVector4 {
        /// <summary>
        /// The X component of the vector.
        /// </summary>
        public int X;

        /// <summary>
        /// The Y component of the vector.
        /// </summary>
        public int Y;

        /// <summary>
        /// The Z component of the vector.
        /// </summary>
        public int Z;

        /// <summary>
        /// The W component of the vector.
        /// </summary>
        public int W;

        /// <summary>
        /// Creates a <see cref="IntVector4" /> with the specified component values.
        /// </summary>
        /// <param name="x">The value to assign to the <see cref="X" /> field.</param>
        /// <param name="y">The value to assign to the <see cref="Y" /> field.</param>
        /// <param name="z">The value to assign to the <see cref="Z" /> field.</param>
        /// <param name="w">The value to assign to the <see cref="W" /> field.</param>
        public IntVector4 (int x, int y, int z, int w) {
            X = x;
            Y = y;
            Z = z;
            W = w;
        }
    }
}