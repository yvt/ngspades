//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System.Numerics;
using System.Runtime.CompilerServices;

namespace Ngs.Utils {
    /// <summary>
    /// Provides extensions methods for vector types in <see cref="System.Numerics" />.
    /// </summary>
    public static class VectorExtensions {
        /// <summary>
        /// Appends a new component to a supplied two-dimensional vector.
        /// </summary>
        /// <param name="v">The input vector.</param>
        /// <param name="t">The value of the new component.</param>
        /// <returns>The newly constructed three-dimensional vector.</returns>
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public static Vector3 Extend(this Vector2 v, float t) => new Vector3(v.X, v.Y, t);

        /// <summary>
        /// Appends a new component to a supplied three-dimensional vector.
        /// </summary>
        /// <param name="v">The input vector.</param>
        /// <param name="t">The value of the new component.</param>
        /// <returns>The newly constructed four-dimensional vector.</returns>
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public static Vector4 Extend(this Vector3 v, float t) => new Vector4(v.X, v.Y, v.Z, t);

        /// <summary>
        /// Removes the last component of a supplied three-dimensional vector.
        /// </summary>
        /// <param name="v">The input vector.</param>
        /// <returns>The newly constructed two-dimensional vector.</returns>
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public static Vector2 Truncate(this Vector3 v) => new Vector2(v.X, v.Y);

        /// <summary>
        /// Removes the last component of a supplied four-dimensional vector.
        /// </summary>
        /// <param name="v">The input vector.</param>
        /// <returns>The newly constructed three-dimensional vector.</returns>
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public static Vector3 Truncate(this Vector4 v) => new Vector3(v.X, v.Y, v.Z);
    }
}