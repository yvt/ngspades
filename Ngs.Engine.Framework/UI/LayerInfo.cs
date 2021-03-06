//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using Ngs.Engine.Presentation;
using Ngs.Engine.Native;
using Ngs.Engine;

namespace Ngs.Engine.UI {
    /// <summary>
    /// Describes the properties of a native layer.
    /// </summary>
    /// <remarks>
    /// <para>This class represents an empty layer. Derive this class and override the
    /// <see cref="UpdateLayer" /> method to generate a layer with graphical contents.
    /// The framework provides the following derived classes:</para>
    /// <list type="bullet">
    ///     <item><term>
    ///     <see cref="SolidColorLayerInfo" /> displays a box filled with a specified color.
    ///     </term></item>
    ///     <item><term>
    ///     <see cref="ImageLayerInfo" /> displays an image (<see cref="Ngs.Engine.Canvas.Image"/>).
    ///     </term></item>
    ///     <item><term>
    ///     <see cref="CanvasLayerInfo" /> has abstract methods that can be implemented to draw
    ///     custom graphical contents using <see cref="Ngs.Engine.Canvas.Painter" />.
    ///     </term></item>
    /// </list>
    /// </remarks>
    public class LayerInfo {
        /// <summary>
        /// Sets or retrieves the bounding rectangle of the contents or an intermediate raster
        /// image.
        /// </summary>
        /// <returns>The bounding rectangle of the contents or an intermediate raster image.</returns>
        public Box2 Bounds { get; set; }

        /// <summary>
        /// Sets or retrieves the opacity of the layer.
        /// </summary>
        /// <returns>The opacity specified in range <c>[0, 1]</c></returns>
        public float Opacity { get; set; } = 1;

        /// <summary>
        /// Sets or retrieves the transformation matrix of the layer.
        /// </summary>
        /// <returns>The transformation matrix.</returns>
        public Matrix4 Transform { get; set; } = Matrix4.Identity;

        /// <summary>
        /// Sets or retrieves the layer flags.
        /// </summary>
        /// <returns>The layer flags.</returns>
        public LayerFlags Flags { get; set; }

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
                if (Transform != Matrix4.Identity) {
                    layer.Transform = Transform;
                }
                layer.Flags = Flags;
            } else {
                if (Bounds != previous.Bounds) {
                    layer.Bounds = Bounds;
                }
                if (Opacity != previous.Opacity) {
                    layer.Opacity = Opacity;
                }
                if (Transform != previous.Transform) {
                    layer.Transform = Transform;
                }
                if (Flags != previous.Flags) {
                    layer.Flags = Flags;
                }
            }
        }
    }
}
