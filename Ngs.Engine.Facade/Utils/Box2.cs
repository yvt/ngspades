//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System.Numerics;
using System.Runtime.InteropServices;

namespace Ngs.Utils {
    /// <summary>
    /// Represents a two-dimensional axis-aligned bounding box (AABB).
    /// </summary>
    /// <remarks>
    /// An AABB is represented by the minimum and maximum (corner) coordinates.
    /// </remarks>
    [StructLayout (LayoutKind.Sequential)]
    public struct Box2 {
        /// <summary>
        /// The minimum coordinate.
        /// </summary>
        public Vector2 Min;

        /// <summary>
        /// The maximum coordinate.
        /// </summary>
        public Vector2 Max;

        /// <summary>
        /// Creates a new <see cref="Box2" /> with given corner coordinates.
        /// </summary>
        /// <param name="min">The minimum coordinate.</param>
        /// <param name="max">The maximum coordinate.</param>
        public Box2 (Vector2 min, Vector2 max) {
            Min = min;
            Max = max;
        }
    }
}