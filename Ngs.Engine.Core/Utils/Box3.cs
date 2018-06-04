//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System.Numerics;
using System.Runtime.InteropServices;

namespace Ngs.Utils {
    /// <summary>
    /// Represents a three-dimensional axis-aligned bounding box (AABB).
    /// </summary>
    /// <remarks>
    /// An AABB is represented by the minimum and maximum (corner) coordinates.
    /// </remarks>
    [StructLayout (LayoutKind.Sequential)]
    public struct Box3 {
        /// <summary>
        /// The minimum coordinate.
        /// </summary>
        public Vector3 Min;

        /// <summary>
        /// The maximum coordinate.
        /// </summary>
        public Vector3 Max;

        /// <summary>
        /// Creates a new <see cref="Box3" /> with given corner coordinates.
        /// </summary>
        /// <param name="min">The minimum coordinate.</param>
        /// <param name="max">The maximum coordinate.</param>
        public Box3 (Vector3 min, Vector3 max) {
            Min = min;
            Max = max;
        }
    }
}