//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System.Runtime.InteropServices;

namespace Ngs.Utils {
    /// <summary>
    /// Represents a vector with three integer values.
    /// </summary>
    [StructLayout (LayoutKind.Sequential)]
    public struct IntVector3 {
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
        /// Creates a <see cref="IntVector3" /> with the specified component values.
        /// </summary>
        /// <param name="x">The value to assign to the <see cref="X" /> field.</param>
        /// <param name="y">The value to assign to the <see cref="Y" /> field.</param>
        /// <param name="z">The value to assign to the <see cref="Z" /> field.</param>
        public IntVector3 (int x, int y, int z) {
            X = x;
            Y = y;
            Z = z;
        }
    }
}