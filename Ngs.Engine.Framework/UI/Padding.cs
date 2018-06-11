//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;

namespace Ngs.Engine.UI {
    /// <summary>
    /// Represents a set of padding values of a box.
    /// </summary>
    public struct Padding {
        /// <summary>
        /// Sets or retrieves the padding value for the left edge.
        /// </summary>
        /// <returns>The padding value, measured in device independent pixels.</returns>
        public float Left { get; set; }
        /// <summary>
        /// Sets or retrieves the padding value for the top edge.
        /// </summary>
        /// <returns>The padding value, measured in device independent pixels.</returns>
        public float Top { get; set; }
        /// <summary>
        /// Sets or retrieves the padding value for the right edge.
        /// </summary>
        /// <returns>The padding value, measured in device independent pixels.</returns>
        public float Right { get; set; }
        /// <summary>
        /// Sets or retrieves the padding value for the bottom edge.
        /// </summary>
        /// <returns>The padding value, measured in device independent pixels.</returns>
        public float Bottom { get; set; }

        /// <summary>
        /// Initializes a new instance of <see cref="Padding" /> with a single padding value applied
        /// to all edges.
        /// </summary>
        /// <param name="ltrb">The padding value to be applied to all edges.</param>
        public Padding(float ltrb) {
            Left = Top = Right = Bottom = ltrb;
        }

        /// <summary>
        /// Initializes a new instance of <see cref="Padding" /> with two padding values each of
        /// which is applied to the horizontial and vertical edges, respectively.
        /// </summary>
        /// <param name="topBottom">The padding value to be applied to the top and bottom
        /// edges.</param>
        /// <param name="leftRight">The padding value to be applied to the left and right
        /// edges.</param>
        public Padding(float topBottom, float leftRight) {
            Top = Bottom = topBottom;
            Left = Right = leftRight;
        }

        /// <summary>
        /// Initializes a new instance of <see cref="Padding" /> with an individual padding value
        /// for each edge.
        /// </summary>
        /// <param name="left">The padding value to be applied to the left edge.</param>
        /// <param name="top">The padding value to be applied to the top edge.</param>
        /// <param name="right">The padding value to be applied to the right edge.</param>
        /// <param name="bottom">The padding value to be applied to the bottom edge.</param>
        public Padding(float left, float top, float right, float bottom) {
            Left = left;
            Top = top;
            Right = right;
            Bottom = bottom;
        }

        /// <summary>
        /// Overrides <see cref="System.Object.Equals(object)" />.
        /// </summary>
        public override bool Equals(object obj) {
            if (obj is Padding other) {
                return Left == other.Left && Top == other.Top &&
                Right == other.Right && Bottom == other.Bottom;
            } else {
                return false;
            }
        }

        /// <summary>
        /// Overrides <see cref="System.Object.GetHashCode" />.
        /// </summary>
        public override int GetHashCode() => unchecked(
            Left.GetHashCode() ^ (Top.GetHashCode() * 3) ^
            (Right.GetHashCode() * 5) ^ (Bottom.GetHashCode() * 11)
        );

        /// <summary>
        /// Overrides <see cref="System.Object.ToString" />.
        /// </summary>
        public override string ToString() {
            return $"Left = {Left}, Top = {Top}, Right = {Right}, Bottom = {Bottom}";
        }

        /// <summary>
        /// Returns the sum of <see cref="Left" /> and <see cref="Right" />.
        /// </summary>
        /// <returns>The sum of <see cref="Left" /> and <see cref="Right" />.</returns>
        public float TotalHorizontalSpacing => Left + Right;

        /// <summary>
        /// Returns the sum of <see cref="Top" /> and <see cref="Bottom" />.
        /// </summary>
        /// <returns>The sum of <see cref="Top" /> and <see cref="Bottom" />.</returns>
        public float TotalVerticalSpacing => Top + Bottom;

        /// <summary>
        /// Returns a <see cref="Vector2" /> value each of whose component contains the sum of the
        /// spacing values of the corresponding axis.
        /// </summary>
        /// <remarks>
        /// The returned value, for example, can be used to calculate the outer size after adding
        /// padding to it as shown in the following expression:
        /// <c>innerSize + padding.TotalAxialSpacing</c>.
        /// </remarks>
        /// <returns>A <see cref="Vector2" /> value each of whose component contains the sum of the
        /// spacing values of the corresponding axis.</returns>
        public Vector2 TotalAxialSpacing => new Vector2(TotalHorizontalSpacing, TotalVerticalSpacing);

        /// <summary>
        /// Adds padding to a supplied <see cref="Box2" /> and returns the <see cref="Box2" />
        /// including the padding.
        /// </summary>
        /// <param name="box">The <see cref="Box2" /> value to add padding to.</param>
        /// <returns>The newly created <see cref="Box2" /> value that includes padding.</returns>
        public Box2 Pad(Box2 box) => new Box2(
            box.Min - new Vector2(Left, Top),
            box.Max + new Vector2(Right, Bottom)
        );

        /// <summary>
        /// Removes padding from a supplied <see cref="Box2" /> and returns the <see cref="Box2" />
        /// not including the padding.
        /// </summary>
        /// <param name="box">The <see cref="Box2" /> value to remove padding from.</param>
        /// <returns>The newly created <see cref="Box2" /> value that does not include padding.
        /// </returns>
        public Box2 Unpad(Box2 box) => new Box2(
            box.Min + new Vector2(Left, Top),
            box.Max - new Vector2(Right, Bottom)
        );
    }
}