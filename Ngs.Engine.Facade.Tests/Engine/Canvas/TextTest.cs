//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using System.IO;
using Ngs.Utils;
using Ngs.Engine.Canvas.Text;
using Xunit;

namespace Ngs.Engine.Canvas.Tests {
    public class TextTest {
        private FontConfig CreateFontConfig() {
            byte[] ReadAllBytes(Stream s) {
                using (var ms = new MemoryStream()) {
                    s.CopyTo(ms);
                    return ms.ToArray();
                }
            }

            var assembly = typeof(TextTest).Assembly;
            var fontData = ReadAllBytes(assembly.GetManifestResourceStream("Fonts.BehdadRegular"));
            var font = new Font(fontData);

            var config = new FontConfig();
            config.AddFontFace(font.FontFaces[0], "Behdad", FontStyle.Normal, 300);

            return config;
        }

        private Painter CreatePainterFromBitmap() {
            var bmp = new Bitmap(new IntVector2(256, 256), PixelFormat.SrgbRgba8);
            return bmp.CreatePainter();
        }

        [Fact]
        public void CanCreateFontConfig() {
            new FontConfig();
        }

        [Fact]
        public void CanLoadFont() {
            CreateFontConfig();
        }

        [Fact]
        public void CanRenderFont() {
            var config = CreateFontConfig();
            var layout = config.LayoutString("مرحبا", new ParagraphStyle());

            using (var painter = CreatePainterFromBitmap()) {
                painter.Translate(20, 240);
                painter.Scale(2);
                painter.SetFillColor(Rgba.White);
                painter.FillTextLayout(layout);
            }
        }
    }
}
