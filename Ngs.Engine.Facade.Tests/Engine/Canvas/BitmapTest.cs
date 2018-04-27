//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using Ngs.Utils;
using Xunit;

namespace Ngs.Engine.Canvas.Tests {
    public class BitmapTest {
        [Fact]
        public void Create() {
            var bmp = new Bitmap(new IntVector2(64, 64), PixelFormat.SrgbRgba8);
            Assert.Equal(new IntVector2(64, 64), bmp.Size);
            Assert.Equal(PixelFormat.SrgbRgba8, bmp.Format);
        }

        [Fact]
        public void Lock() {
            var bmp = new Bitmap(new IntVector2(64, 64), PixelFormat.SrgbRgba8);
            bmp.Lock((Bitmap, span) => {
                Assert.Equal(64 * 64 * 4, span.Length);
                var rng = new Random(1);
                for (int i = 0; i < span.Length; ++i) {
                    span[i] = (byte)rng.Next();
                }
                return false;
            });
            bmp.Lock((Bitmap, span) => {
                Assert.Equal(64 * 64 * 4, span.Length);
                var rng = new Random(1);
                for (int i = 0; i < span.Length; ++i) {
                    Assert.Equal((byte)rng.Next(), span[i]);
                }
                return false;
            });
        }

        [Fact]
        public void Clonable() {
            var bmp = new Bitmap(new IntVector2(64, 64), PixelFormat.SrgbRgba8);
            bmp.Lock((Bitmap, span) => {
                Assert.Equal(64 * 64 * 4, span.Length);
                var rng = new Random(1);
                for (int i = 0; i < span.Length; ++i) {
                    span[i] = (byte)rng.Next();
                }
                return false;
            });
            bmp = bmp.Clone();
            bmp.Lock((Bitmap, span) => {
                Assert.Equal(64 * 64 * 4, span.Length);
                var rng = new Random(1);
                for (int i = 0; i < span.Length; ++i) {
                    Assert.Equal((byte)rng.Next(), span[i]);
                }
                return false;
            });
        }

        [Fact]
        public void IntoImage() {
            var bmp = new Bitmap(new IntVector2(64, 64), PixelFormat.SrgbRgba8);
            Assert.NotNull(bmp.IntoImage());
        }

        [Fact]
        public void ToImage() {
            var bmp = new Bitmap(new IntVector2(64, 64), PixelFormat.SrgbRgba8);
            Assert.NotNull(bmp.ToImage());
            Assert.Equal(new IntVector2(64, 64), bmp.Size);
        }
    }
}
