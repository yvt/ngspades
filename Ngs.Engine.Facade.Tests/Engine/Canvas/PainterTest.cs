//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using Ngs.Utils;
using Xunit;

namespace Ngs.Engine.Canvas.Tests {
    public class PainterTest {
        private IPainter CreateFromBitmap() {
            var bmp = new Bitmap(new IntVector2(256, 256), PixelFormat.SrgbRgba8);
            return bmp.CreatePainter();
        }

        [Fact]
        public void CanCreate() {
            CreateFromBitmap();
        }

        [Fact]
        public void CanTranslate() {
            CreateFromBitmap().Translate(new Vector2(0, 0));
        }

        [Fact]
        public void CanNonUniformScale() {
            CreateFromBitmap().NonUniformScale(4.0f, 0.5f);
        }

        [Fact]
        public void CanSetFillColor() {
            CreateFromBitmap().SetFillColor(Rgba.White);
        }
    }
}