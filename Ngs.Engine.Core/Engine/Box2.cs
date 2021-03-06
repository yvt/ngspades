//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System.Numerics;
using System.Runtime.InteropServices;

namespace Ngs.Engine {
    /// <summary>
    /// Represents a two-dimensional axis-aligned bounding box (AABB).
    /// </summary>
    /// <remarks>
    /// An AABB is represented by the minimum and maximum (corner) coordinates.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
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
        public Box2(Vector2 min, Vector2 max) {
            Min = min;
            Max = max;
        }

        /// <summary>
        /// Creates a new <see cref="Box2" /> with given top-left coordinates
        /// and extents.
        /// </summary>
        /// <param name="left">The X coordinate of the left edge of the box.
        /// <c>Min.X</c> is initialized to this value.</param>
        /// <param name="top">The X coordinate of the top edge of the box.
        /// <c>Min.Y</c> is initialized to this value.</param>
        /// <param name="width">The width (size in the X direction) of the box.
        /// <c>Max.X</c> is calculated by adding <see paramref="left" /> to this value.</param>
        /// <param name="height">The height (size in the Y direction) of the box.
        /// <c>Max.Y</c> is calculated by adding <see paramref="top" /> to this value.</param>
        public Box2(float left, float top, float width, float height) :
            this(new Vector2(left, top), new Vector2(left + width, top + height)) {
        }

        /// <summary>
        /// Retrieves the width (size in the X direction) of the box.
        /// </summary>
        /// <remarks>
        /// The width of a box is calculated by <c>Max.X - Min.X</c>.
        /// </remarks>
        /// <returns>The width (size in the X direction) of the box.</returns>
        public float Width { get => Max.X - Min.X; }

        /// <summary>
        /// Retrieves the height (size in the Y direction) of the box.
        /// </summary>
        /// <remarks>
        /// The height of a box is calculated by <c>Max.Y - Min.Y</c>.
        /// </remarks>
        /// <returns>The height (size in the Y direction) of the box.</returns>
        public float Height { get => Max.Y - Min.Y; }

        /// <summary>
        /// Retrieves the size of the box.
        /// </summary>
        /// <returns>The size of the box.</returns>
        public Vector2 Size { get => Max - Min; }

        /// <summary>
        /// Sets (without moving the edge of the opposite side) or retrieves the
        /// X coordinate of the left edge of the box.
        /// </summary>
        /// <remarks>
        /// This property is equivalent to <c>Min.X</c>.
        /// </remarks>
        /// <returns>The X coordinate of the left edge of the box.</returns>
        public float Left {
            get => Min.X;
            set => Min.X = value;
        }

        /// <summary>
        /// Sets (without moving the edge of the opposite side) or retrieves the
        /// X coordinate of the right edge of the box.
        /// </summary>
        /// <remarks>
        /// This property is equivalent to <c>Max.X</c>.
        /// </remarks>
        /// <returns>The X coordinate of the ригчт edge of the box.</returns>
        public float Right {
            get => Max.X;
            set => Max.X = value;
        }

        /// <summary>
        /// Sets (without moving the edge of the opposite side) or retrieves the
        /// Y coordinate of the top edge of the box.
        /// </summary>
        /// <remarks>
        /// This property is equivalent to <c>Min.Y</c>.
        /// </remarks>
        /// <returns>The Y coordinate of the top edge of the box.</returns>
        public float Top {
            get => Min.Y;
            set => Min.Y = value;
        }

        /// <summary>
        /// Sets (without moving the edge of the opposite side) or retrieves the
        /// Y coordinate of the bottom edge of the box.
        /// </summary>
        /// <remarks>
        /// This property is equivalent to <c>Max.Y</c>.
        /// </remarks>
        /// <returns>The Y coordinate of the bottom edge of the box.</returns>
        public float Bottom {
            get => Max.Y;
            set => Max.Y = value;
        }

        /// <summary>
        /// Overrides <see cref="System.Object.Equals(object)" />.
        /// </summary>
        public override bool Equals(object obj) {
            if (obj is Box2 o) {
                return this == o;
            } else {
                return false;
            }
        }

        /// <summary>
        /// Overrides <see cref="System.Object.GetHashCode()" />.
        /// </summary>
        public override int GetHashCode() => unchecked(Min.GetHashCode() ^ (Max.GetHashCode() * 6));

        /// <summary>
        /// Indicates whether two values of this type are equal.
        /// </summary>
        /// <param name="a">The first operand.</param>
        /// <param name="b">The second operand.</param>
        /// <returns><c>true</c> if <paramref name="a" /> is equal to <paramref name="b" />;
        /// otherwise; <c>false</c>.</returns>
        public static bool operator ==(Box2 a, Box2 b) => a.Min == b.Min && a.Max == b.Max;

        /// <summary>
        /// Indicates whether two values of this type are not equal.
        /// </summary>
        /// <param name="a">The first operand.</param>
        /// <param name="b">The second operand.</param>
        /// <returns><c>true</c> if <paramref name="a" /> is not equal to <paramref name="b" />;
        /// otherwise; <c>false</c>.</returns>
        public static bool operator !=(Box2 a, Box2 b) => a.Min != b.Min || a.Max != b.Max;

        /// <summary>
        /// Retrieves a normalized (having non-negative width and height) rectangle.
        /// </summary>
        /// <remarks>
        /// A normalized rectangle is created by exchanging the values of <see cref="Min" /> and
        /// <see cref="Max" /> as needed so they correctly represent the minimum and maximum
        /// coordinates.
        /// The resulting rectangle has the identical set of corner points as the original
        /// rectangle does but has non-negative width and height values.
        /// </remarks>
        /// <returns>The normalized rectangle.</returns>
        public Box2 Normalized => new Box2(Vector2.Min(Min, Max), Vector2.Max(Min, Max));

        /// <summary>
        /// Indicates whether the area represented by this <see cref="Box2" /> is empty.
        /// </summary>
        /// <returns><c>true</c> if either of this rectangle's width and height is not positive;
        /// otherwise, <c>false</c>.</returns>
        /// <seealso cref="Normalized" />
        public bool IsEmpty => Max.X <= Min.X || Max.Y <= Min.Y;

        /// <summary>
        /// Indicates whether the rectangle is normalized (having non-negative width and height).
        /// </summary>
        /// <returns><c>true</c> if this rectangle has non-negative width and height);
        /// <c>false</c> otherwise.</returns>
        /// <seealso cref="Normalized" />
        public bool IsValid => Max.X >= Min.X && Max.Y >= Min.Y;

        /// <summary>
        /// Creates a new <see cref="Box2" /> by inflating it by a specified amount.
        /// </summary>
        /// <param name="amount">The amount to inflate this box.</param>
        /// <returns>The new <see cref="Box2" />.</returns>
        public Box2 GetInflated(float amount) =>
            new Box2(Min - new Vector2(amount), Max + new Vector2(amount));

        /// <summary>
        /// Creates a new <see cref="Box2" /> by translating it by a specified displacement vector.
        /// </summary>
        /// <param name="v">The translation amount.</param>
        /// <returns>The new <see cref="Box2" />.</returns>
        public Box2 GetTranslated(Vector2 v) => new Box2(Min + v, Max + v);

        /// <summary>
        /// Creates a new <see cref="Box2" /> by translating it by a specified displacement vector.
        /// </summary>
        /// <param name="x">The translation amount in the X direction.</param>
        /// <param name="y">The translation amount in the Y direction.</param>
        /// <returns>The new <see cref="Box2" />.</returns>
        public Box2 GetTranslated(float x, float y) => GetTranslated(new Vector2(x, y));
    }
}