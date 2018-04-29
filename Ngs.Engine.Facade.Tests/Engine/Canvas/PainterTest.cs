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
        private Painter CreateFromBitmap() {
            var bmp = new Bitmap(new IntVector2(256, 256), PixelFormat.SrgbRgba8);
            return bmp.CreatePainter();
        }

        [Fact]
        public void CanCreate() {
            using (var _painter = CreateFromBitmap()) { }
        }

        [Fact]
        public void CanTranslate() {
            using (var painter = CreateFromBitmap()) {
                painter.Translate(2.0f, 4.0f);
            }
        }

        [Fact]
        public void CanScale() {
            using (var painter = CreateFromBitmap()) {
                painter.Scale(4.0f, 0.5f);
                painter.Scale(2.0f);
            }
        }

        [Fact]
        public void CanSetFillColor() {
            using (var painter = CreateFromBitmap()) {
                painter.SetFillColor(Rgba.White);
            }
        }

        [Fact]
        public void CanLockAndSetFillColor() {
            using (var painter = CreateFromBitmap())
            using (var _guard = painter.Lock()) {
                painter.SetFillColor(Rgba.White);
            }
        }
    }
}