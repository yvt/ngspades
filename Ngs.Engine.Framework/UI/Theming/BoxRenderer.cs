//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using Ngs.Engine;

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
    }
}
