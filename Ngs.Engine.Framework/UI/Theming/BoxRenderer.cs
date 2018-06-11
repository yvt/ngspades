//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using Ngs.Engine;
using Ngs.Engine.Canvas;
using Ngs.Engine.UI.Utils;

namespace Ngs.Engine.UI.Theming {
    /// <summary>
    /// Provides methods for rendering boxes in various styles using a supplied
    /// <see cref="View.RenderContext" /> object.
    /// </summary>
    public static class BoxRenderer {
        /// <summary>
        /// Emits a layer containing a filled box.
        /// </summary>
        /// <param name="context">The <see cref="View.RenderContext" /> object used to emit
        /// layers.</param>
        /// <param name="bounds">The bounding box of the box to be rendered.</param>
        /// <param name="fillColor">The interior color of the box to be rendered.</param>
        public static void EmitBoxFill(View.RenderContext context, Box2 bounds, Rgba fillColor) {
            context.EmitLayer(new SolidColorLayerInfo()
            {
                Bounds = bounds,
                FillColor = fillColor,
            });
        }

        /// <summary>
        /// Emits a layer set containing the stroke of a box.
        /// </summary>
        /// <param name="context">The <see cref="View.RenderContext" /> object used to emit
        /// layers.</param>
        /// <param name="bounds">The bounding box of the box to be rendered.</param>
        /// <param name="strokeColor">The color of the box to be rendered.</param>
        /// <param name="width">The width of the stroked line. Must not be negative.</param>
        public static void EmitBoxStroke(View.RenderContext context, Box2 bounds, Rgba strokeColor, float width) {
            if (!(width >= 0)) {
                throw new ArgumentOutOfRangeException(nameof(width));
            }
            bounds = bounds.Normalized;

            if (width <= 0) {
                // Make sure to always emit the same number of layers
                context.EmitLayer(null); context.EmitLayer(null);
                context.EmitLayer(null); context.EmitLayer(null);
                return;
            }

            if (bounds.Width < width * 2 || bounds.Height < width * 2) {
                // The bounding rect is narrower than the stroke width
                context.EmitLayer(new SolidColorLayerInfo()
                {
                    Bounds = bounds,
                    FillColor = strokeColor,
                });
                context.EmitLayer(null); context.EmitLayer(null); context.EmitLayer(null);
                return;
            }

            // Render the stroke of a box using four filled rectangles. This has a T-junction
            // issue but should be okay in practice.
            context.EmitLayer(new SolidColorLayerInfo()
            {
                Bounds = new Box2(bounds.Min, new Vector2(bounds.Max.X, bounds.Min.Y + width)),
                FillColor = strokeColor,
            });
            context.EmitLayer(new SolidColorLayerInfo()
            {
                Bounds = new Box2(new Vector2(bounds.Min.X, bounds.Max.Y - width), bounds.Max),
                FillColor = strokeColor,
            });
            context.EmitLayer(new SolidColorLayerInfo()
            {
                Bounds = new Box2(
                    new Vector2(bounds.Min.X, bounds.Min.Y + width),
                    new Vector2(bounds.Min.X + width, bounds.Max.Y - width)
                ),
                FillColor = strokeColor,
            });
            context.EmitLayer(new SolidColorLayerInfo()
            {
                Bounds = new Box2(
                    new Vector2(bounds.Max.X - width, bounds.Min.Y + width),
                    new Vector2(bounds.Max.X, bounds.Max.Y - width)
                ),
                FillColor = strokeColor,
            });
        }

        static class BlurredBox {
            public readonly static Image Image;
            public const float Resolution = 32.0f;
            public const float Width = 3.0f; // ratio to `Resolution`

            /// <summary>
            /// The number of pixels for each side of the bitmap.
            /// </summary>
            const int BitmapSize = (int)(Resolution * Width * 2);

            static BlurredBox() {
                var bitmap = new Bitmap(new IntVector2(BitmapSize, BitmapSize), PixelFormat.SrgbRgba8);
                bitmap.UsingContents<object>((_, data) => {
                    Span<float> side = stackalloc float[BitmapSize];

                    for (int i = 0; i < BitmapSize; ++i) {
                        float x = (float)i * (1 / Resolution);
                        float val = (float)MathUtils.Erf(Width - x) * 0.5f + 0.5f;
                        side[i] = val;
                    }

                    int index = 3;
                    for (int y = 0; y < BitmapSize; ++y) {
                        for (int x = 0; x < BitmapSize; ++x) {
                            float val = side[x] * side[y];
                            data[index] = (byte)(val * 255);
                            index += 4;
                        }
                    }

                    return null;
                });

                Image = bitmap.IntoImage();
            }
        }

        /// <summary>
        /// Emits a layer containing a blurred, filled box.
        /// </summary>
        /// <param name="context">The <see cref="View.RenderContext" /> object used to emit
        /// layers.</param>
        /// <param name="bounds">The bounding box of the box to be rendered.</param>
        /// <param name="opacity">The opacity of the box to be rendered.</param>
        /// <param name="radius">The blur radius. Must not be negative.</param>
        public static void EmitBoxFillShadow(View.RenderContext context, Box2 bounds, float opacity, float radius) {
            if (!(radius >= 0)) {
                throw new ArgumentOutOfRangeException(nameof(radius));
            }

            // exp(-x^2) is sufficiently small for `x > 2`, so map `[-radius, radius]` to
            // `[-2, 2]` in the blur kernel's space.
            radius *= 1 / 2f;

            bounds = bounds.Normalized;
            Vector2 relativeSize = bounds.Size * (0.5f / radius);
            if (Math.Max(relativeSize.X, relativeSize.Y) > 1e10) {
                // Guard against extremely large bounds and/or small (or zero) `radius`
                context.EmitLayer(new SolidColorLayerInfo()
                {
                    Bounds = bounds,
                    FillColor = new Rgba(0, 0, 0, opacity),
                });

                // Make sure to always emit the same number of layers
                context.EmitLayer(null); context.EmitLayer(null); context.EmitLayer(null);
                return;
            }

            Vector2 center = (bounds.Min + bounds.Max) * 0.5f;

            // Approximate `erf(x+w) - erf(x-w)` for each axis (where `w = relativeSize.{X,Y}`)
            // by transforming `erf(x)`. Down below is where the magic happens.
            const float a1 = 1.06527f, a2 = 1.13511f, a3 = -3.1682f;
            Vector2 erfOffset = default;
            Vector2 erfDensity = default;

            for (int axis = 0; axis < 2; ++axis) {
                float w = relativeSize.GetElementAt(axis);
                erfOffset.ElementAt(axis) = w + a1 - a1 / (1 + a2 * MathF.Exp(a3 * w));
                erfDensity.ElementAt(axis) = 2 / (1 + MathF.Exp(w * (-10 / 3f))) - 1;
            }

            opacity *= erfDensity.X * erfDensity.Y;

            // Render pieces of transformed `erf(x) * erf(y)`
            var source = BlurredBox.Image;
            Vector2 sourceOrigin = new Vector2(BlurredBox.Width * BlurredBox.Resolution);
            Vector2 sourceInset = erfOffset * BlurredBox.Resolution;
            Vector2 sourceOuterCorner = new Vector2(BlurredBox.Width) * BlurredBox.Resolution;
            Box2 sourceBox = new Box2(
                sourceOrigin - sourceInset,
                sourceOrigin + sourceOuterCorner
            );

            Vector2 layerSize = (erfOffset + new Vector2(BlurredBox.Width)) * radius;

            for (int i = 0; i < 4; ++i) {
                // For each quadrant...
                bool ix = (i & 1) != 0, iy = (i & 2) != 0;
                float fx = ix ? 1f : -1f, fy = iy ? 1f : -1f;
                Vector2 dvec = new Vector2(fx, fy);

                context.EmitLayer(new ImageLayerInfo()
                {
                    Bounds = new Box2(center, center + layerSize * dvec),
                    Image = source,
                    Source = sourceBox,
                    Opacity = opacity,
                });
            }
        }
    }
}
