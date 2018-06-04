//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using Ngs.Engine.Canvas.Text;
using Ngs.Engine;

namespace Ngs.Engine.UI.Widgets {
    sealed class TextLayoutLayerInfo : CanvasLayerInfo {
        readonly TextLayout textLayout;
        readonly Rgba color;

        public TextLayoutLayerInfo(TextLayout textLayout, Rgba color) {
            this.textLayout = textLayout;
            this.color = color;
        }

        protected override Box2 ContentBounds => textLayout.VisualBounds;

        protected override void PaintContents(in PaintParams p) {
            p.Painter.SetFillColor(color);
            p.Painter.FillTextLayout(textLayout);
        }

        protected override bool ShouldUpdate(LayerInfo previous) {
            var p = (TextLayoutLayerInfo)previous;
            return textLayout != p.textLayout || color != p.color;
        }
    }
}
