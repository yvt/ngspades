//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System.Runtime.InteropServices;

namespace Ngs.Engine {
    /// <summary>
    /// Represents a color value using four single precision floating point
    /// numbers.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct Rgba {
        /// <summary>
        /// The red component.
        /// </summary>
        public float Red;

        /// <summary>
        /// The green component.
        /// </summary>
        public float Green;

        /// <summary>
        /// The blue component.
        /// </summary>
        public float Blue;

        /// <summary>
        /// The alpha component.
        /// </summary>
        public float Alpha;

        /// <summary>
        /// Creates a <see cref="Rgba" /> with the specified component values.
        /// </summary>
        /// <param name="red">The value to assign to the <see cref="Red" /> field.</param>
        /// <param name="green">The value to assign to the <see cref="Green" /> field.</param>
        /// <param name="blue">The value to assign to the <see cref="Blue" /> field.</param>
        /// <param name="alpha">The value to assign to the <see cref="Alpha" /> field.</param>
        public Rgba(float red, float green, float blue, float alpha) {
            Red = red;
            Green = green;
            Blue = blue;
            Alpha = alpha;
        }

        /// <summary>
        /// Retrieves a <see cref="Rgba" /> representing an opaque black color.
        /// </summary>
        public static readonly Rgba Black = new Rgba(0, 0, 0, 1);

        /// <summary>
        /// Retrieves a <see cref="Rgba" /> representing an opaque white color.
        /// </summary>
        public static readonly Rgba White = new Rgba(1, 1, 1, 1);

        /// <summary>
        /// Retrieves a <see cref="Rgba" /> representing a transparent black color.
        /// </summary>
        public static readonly Rgba TransparentBlack = new Rgba(0, 0, 0, 0);

        /// <summary>
        /// Retrieves a <see cref="Rgba" /> representing a transparent white color.
        /// </summary>
        public static readonly Rgba TransparentWhite = new Rgba(1, 1, 1, 0);

        /// <summary>
        /// Overrides <see cref="System.Object.Equals(object)" />.
        /// </summary>
        public override bool Equals(object obj) {
            if (obj is Rgba o) {
                return this == o;
            } else {
                return false;
            }
        }

        /// <summary>
        /// Overrides <see cref="System.Object.GetHashCode()" />.
        /// </summary>
        public override int GetHashCode() => unchecked(
            Red.GetHashCode() ^ (Green.GetHashCode() * 3) ^
            (Blue.GetHashCode() * 7) ^ (Alpha.GetHashCode() * 11)
        );

        /// <summary>
        /// Indicates whether two values of this type are equal.
        /// </summary>
        /// <param name="a">The first operand.</param>
        /// <param name="b">The second operand.</param>
        /// <returns><c>true</c> if <paramref name="a" /> is equal to <paramref name="b" />;
        /// otherwise; <c>false</c>.</returns>
        public static bool operator ==(Rgba a, Rgba b) =>
            a.Red == b.Red && a.Green == b.Green && a.Blue == b.Blue && a.Alpha == b.Alpha;

        /// <summary>
        /// Indicates whether two values of this type are not equal.
        /// </summary>
        /// <param name="a">The first operand.</param>
        /// <param name="b">The second operand.</param>
        /// <returns><c>true</c> if <paramref name="a" /> is not equal to <paramref name="b" />;
        /// otherwise; <c>false</c>.</returns>
        public static bool operator !=(Rgba a, Rgba b) =>
            a.Red != b.Red || a.Green != b.Green || a.Blue != b.Blue || a.Alpha != b.Alpha;

    }
}