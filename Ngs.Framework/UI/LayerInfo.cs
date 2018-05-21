//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using Ngs.Engine.Presentation;
using Ngs.Engine.Native;
using Ngs.Utils;

namespace Ngs.UI {
    // TODO: More `LayerInfo` derived classes

    /// <summary>
    /// Describes the properties of a native layer.
    /// </summary>
    public class LayerInfo {
        public Box2 Bounds { get; set; }

        public float Opacity { get; set; } = 1;

        /// <summary>
        /// Updates the properties of a supplied native layer.
        /// </summary>
        /// <remarks>
        /// <para>The implementation calculates the difference between this <see cref="LayerInfo" />
        /// and the previous one (<paramref name="previous" />), and updates only the changed
        /// portion of the native layer properties.</para>
        /// <para>The following layer properties are not affected by this method:</para>
        /// <list type="bullet">
        ///     <item><term>
        ///     <see cref="INgsPFLayer.Mask" />
        ///     </term></item>
        ///     <item><term>
        ///     <see cref="INgsPFLayer.Child" />
        ///     </term></item>
        /// </list>
        /// </remarks>
        /// <param name="layer">The native layer whose properties are being updated.</param>
        /// <param name="previous">The <see cref="LayerInfo" /> that was previously used to set
        /// <paramref name="layer" />'s properties. Can be <c>null</c>. If not <c>null</c>, it must
        /// have the same dynamic type as <c>this</c>.</param>
        public virtual void UpdateLayer(INgsPFLayer layer, LayerInfo previous) {
            if (previous == null) {
                layer.Bounds = Bounds;
                layer.Opacity = Opacity;
            } else {
                if (Bounds != previous.Bounds) {
                    layer.Bounds = Bounds;
                }
                if (Opacity != previous.Opacity) {
                    layer.Opacity = Opacity;
                }
            }
        }
    }
}
