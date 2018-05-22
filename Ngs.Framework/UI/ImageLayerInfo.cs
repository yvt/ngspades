//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using Ngs.Engine.Presentation;
using Ngs.Engine.Native;
using Ngs.Engine.Canvas;
using Ngs.Utils;

namespace Ngs.UI {
    /// <summary>
    /// Describes the properties of a native layer with a solid color content.
    /// </summary>
    public sealed class ImageLayerInfo : LayerInfo {
        /// <summary>
        /// Sets or retrieves the image.
        /// </summary>
        /// <returns>The image.</returns>
        public Image Image { get; set; }

        /// <summary>
        /// Sets or retrieves the source rectangle.
        /// </summary>
        /// <returns>The source rectangle.</returns>
        public Box2 Source { get; set; }

        /// <summary>
        /// Sets or retrieves the wrap mode.
        /// </summary>
        /// <returns>The wrap mode.</returns>
        public ImageWrapMode WrapMode { get; set; } = ImageWrapMode.Clamp;

        /// <summary>
        /// Overrides <see cref="LayerInfo.UpdateLayer(INgsPFLayer, LayerInfo)" />.
        /// </summary>
        public override void UpdateLayer(INgsPFLayer layer, LayerInfo previous) {
            if (previous is ImageLayerInfo p) {
                if (Image == p.Image && Source == p.Source && WrapMode == p.WrapMode) {
                    goto skip;
                }
            }

            if (Image != null) {
                layer.SetContentsImage(Image.NativeImage, Source, WrapMode);
            } else {
                layer.SetContentsEmpty();
            }

        skip:
            base.UpdateLayer(layer, previous);
        }
    }
}
