//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;

namespace Ngs.Engine.UI {
    /// <summary>
    /// Represents a result of the measurement step of the layouting algorithm.
    /// </summary>
    public struct Measurement {
        /// <summary>
        /// Sets or retrieves the minimum size.
        /// </summary>
        /// <remarks>
        /// The extent of a view can never be smaller than this value in any axes. The possible
        /// smallest value for this property is <c>(0, 0)</c>.
        /// </remarks>
        /// <returns>The minimum size.</returns>
        public Vector2 MinimumSize { get; set; }

        /// <summary>
        /// Sets or retrieves the maximum size.
        /// </summary>
        /// <remarks>
        /// The extent of a view can never be larger than this value in any axes.
        /// <see cref="Single.PositiveInfinity" /> indicates that there is no upper bound for the
        /// corresponding axis.
        /// </remarks>
        /// <returns>The maximum size.</returns>
        public Vector2 MaximumSize { get; set; }

        /// <summary>
        /// Sets or retrieves the preferred size.
        /// </summary>
        /// <remarks>
        /// <para>The preferred size is used as a hint to distribute the space among views and to
        /// determine the default size when the size is not specified by other means.</para>
        /// </remarks>
        /// <returns>The preferred size.</returns>
        public Vector2 PreferredSize { get; set; }

        /// <summary>
        /// Retrieves the default <see cref="Measurement" /> value.
        /// </summary>
        /// <remarks>
        /// <para>The default value has the following property values:</para>
        /// <list type="bullet">
        ///     <item><term>
        ///         <see cref="MinimumSize" /> is <c>(0, 0)</c>.
        ///     </term></item>
        ///     <item><term>
        ///         <see cref="MaximumSize" /> is <c>(+∞, +∞)</c>. <c>+∞</c> denotes
        ///         <see cref="Single.PositiveInfinity" />
        ///     </term></item>
        ///     <item><term>
        ///         <see cref="PreferredSize" /> is <c>(0, 0)</c>.
        ///     </term></item>
        /// </list>
        /// </remarks>
        /// <returns>The default <see cref="Measurement" /> value.</returns>
        public static readonly Measurement Default = new Measurement()
        {
            MaximumSize = new Vector2(float.PositiveInfinity, float.PositiveInfinity),
        };
    }
}
