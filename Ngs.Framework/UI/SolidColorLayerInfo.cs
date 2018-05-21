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
    /// <summary>
    /// Describes the properties of a native layer with a solid color content.
    /// </summary>
    public class SolidColorLayerInfo : LayerInfo {
        /// <summary>
        /// Sets or retrieves the fill color.
        /// </summary>
        /// <returns>The fill color.</returns>
        public Rgba FillColor { get; set; }

        /// <summary>
        /// Overrides <see cref="LayerInfo.UpdateLayer(INgsPFLayer, LayerInfo)" />.
        /// </summary>
        public override void UpdateLayer(INgsPFLayer layer, LayerInfo previous) {
            if (previous is SolidColorLayerInfo p) {
                if (FillColor != p.FillColor) {
                    layer.SetContentsSolidColor(FillColor);
                }
            } else {
                layer.SetContentsSolidColor(FillColor);
            }

            base.UpdateLayer(layer, previous);
        }
    }
}
