//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System.Runtime.InteropServices;

namespace Ngs.Utils {
    /// <summary>
    /// Represents a color value using four single precision floating point
    /// numbers.
    /// </summary>
    [StructLayout (LayoutKind.Sequential)]
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
        public Rgba (float red, float green, float blue, float alpha) {
            Red = red;
            Green = green;
            Blue = blue;
            Alpha = alpha;
        }
    }
}