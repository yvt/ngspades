//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using System.Runtime.CompilerServices;

namespace Ngs.Engine {
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

        /// <summary>
        /// Retrieves an element of a supplied two-dimensional vector at a specified index.
        /// </summary>
        /// <param name="v">The input vector.</param>
        /// <param name="index">The index of the element to retrieve.</param>
        /// <returns>The copy of an element of <paramref name="v" />.</returns>
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public static float GetElementAt(this Vector2 v, int index) {
            switch (index) {
                case 0: return v.X;
                case 1: return v.Y;
                default: throw new IndexOutOfRangeException();
            }
        }

        /// <summary>
        /// Retrieves a reference to an element of a supplied two-dimensional vector at a
        /// specified index.
        /// </summary>
        /// <param name="v">The input vector.</param>
        /// <param name="index">The index of the element to retrieve.</param>
        /// <returns>The reference to an element of <paramref name="v" />.</returns>
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public static ref float ElementAt(this ref Vector2 v, int index) {
            switch (index) {
                case 0: return ref v.X;
                case 1: return ref v.Y;
                default: throw new IndexOutOfRangeException();
            }
        }

        /// <summary>
        /// Retrieves an element of a supplied three-dimensional vector at a specified index.
        /// </summary>
        /// <param name="v">The input vector.</param>
        /// <param name="index">The index of the element to retrieve.</param>
        /// <returns>The copy of an element of <paramref name="v" />.</returns>
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public static float GetElementAt(this Vector3 v, int index) {
            switch (index) {
                case 0: return v.X;
                case 1: return v.Y;
                case 2: return v.Z;
                default: throw new IndexOutOfRangeException();
            }
        }

        /// <summary>
        /// Retrieves a reference to an element of a supplied three-dimensional vector at a
        /// specified index.
        /// </summary>
        /// <param name="v">The input vector.</param>
        /// <param name="index">The index of the element to retrieve.</param>
        /// <returns>The reference to an element of <paramref name="v" />.</returns>
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public static ref float ElementAt(this ref Vector3 v, int index) {
            switch (index) {
                case 0: return ref v.X;
                case 1: return ref v.Y;
                case 2: return ref v.Z;
                default: throw new IndexOutOfRangeException();
            }
        }

        /// <summary>
        /// Retrieves an element of a supplied four-dimensional vector at a specified index.
        /// </summary>
        /// <param name="v">The input vector.</param>
        /// <param name="index">The index of the element to retrieve.</param>
        /// <returns>The copy of an element of <paramref name="v" />.</returns>
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public static float GetElementAt(this Vector4 v, int index) {
            switch (index) {
                case 0: return v.X;
                case 1: return v.Y;
                case 2: return v.Z;
                case 3: return v.W;
                default: throw new IndexOutOfRangeException();
            }
        }

        /// <summary>
        /// Retrieves a reference to an element of a supplied four-dimensional vector at a
        /// specified index.
        /// </summary>
        /// <param name="v">The input vector.</param>
        /// <param name="index">The index of the element to retrieve.</param>
        /// <returns>The reference to an element of <paramref name="v" />.</returns>
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public static ref float ElementAt(this ref Vector4 v, int index) {
            switch (index) {
                case 0: return ref v.X;
                case 1: return ref v.Y;
                case 2: return ref v.Z;
                case 3: return ref v.W;
                default: throw new IndexOutOfRangeException();
            }
        }
    }
}