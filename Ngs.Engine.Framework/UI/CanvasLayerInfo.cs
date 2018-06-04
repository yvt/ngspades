//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Security;
using System.Numerics;
using Ngs.Engine.Presentation;
using Ngs.Engine.Native;
using Ngs.Engine.Canvas;
using Ngs.Engine;

namespace Ngs.UI {
    /// <summary>
    /// Describes the properties of a native layer with a custom-drawn content.
    /// </summary>
    public abstract class CanvasLayerInfo : LayerInfo {
        /// <summary>
        /// The bounding rectangle of the painted contents.
        /// </summary>
        /// <remarks>
        /// This value is used to compute the size of the backing store of a layer. Graphics drawn
        /// outside this rectangle will be clipped.
        /// </remarks>
        /// <returns>A <see cref="Box2" /> value representing the bounding rectangle.</returns>
        protected abstract Box2 ContentBounds { get; }

        /// <summary>
        /// Sets or retrieves the position of this layer within the parent layer (i.e., the location
        /// where the point <c>(0, 0)</c> of the contents is located).
        /// </summary>
        /// <returns>A position in the parent layer's space.</returns>
        public Vector2 Origin { get; set; }

        /// <summary>
        /// Retrieves the bounding rectangle of the layer.
        /// </summary>
        /// <remarks>
        /// <see cref="CanvasLayerInfo" /> automatically updates the value of this property based on
        /// <see cref="ContentBounds" />. Use the <see cref="Origin" /> property to move the
        /// layer.
        /// </remarks>
        /// <returns>The bounding rectangle of the layer.</returns>
        public new Box2 Bounds { get => base.Bounds; }

        /// <summary>
        /// Indicates whether the contents should be repainted, by comparing this instance to the
        /// previous one (<paramref name="previous" />).
        /// </summary>
        /// <param name="previous">The <see cref="CanvasLayerInfo" /> instance that drew
        /// the previous contents. The dynamic type must match that of <c>this</c>. Must not be
        /// <c>null</c>.</param>
        /// <returns><c>true</c> if the contents should be repainted; otherwise <c>false</c>.
        /// </returns>
        protected abstract bool ShouldUpdate(LayerInfo previous);

        /// <summary>
        /// Encapsulates the information required to repaint the contents of the layer
        /// represented by <see cref="CanvasLayerInfo" />.
        /// </summary>
        public readonly struct PaintParams {
            internal PaintParams(Painter painter, Box2 bounds) {
                this.Painter = painter;
                this.Bounds = bounds;
            }

            /// <summary>
            /// The painter object that should be used to repaint the contents.
            /// </summary>
            /// <returns>A painter object.</returns>
            public Painter Painter { get; }

            /// <summary>
            /// Retrieves the ratio from device independent pixels to physical pixels of the backing
            /// store image.
            /// </summary>
            /// <remarks>
            /// The following factors affect this property: the monitor's DPI scaling config and
            /// the layout transform (TODO) of the associated view and its ancestors.
            /// </remarks>
            /// <returns>The ratio from device independent pixels to physical pixels of the
            /// backing store image for each axis. Currently, this property is not fully implemented
            /// and therefore <c>(1, 1)</c> is always returned.
            /// </returns>
            public Vector2 PixelRatio { get => new Vector2(1, 1); }

            /// <summary>
            /// Retrieves the rectangular region where the contents can be drawn.
            /// </summary>
            /// <remarks>
            /// This value is computed from <see cref="CanvasLayerInfo.ContentBounds" />.
            /// </remarks>
            /// <returns>A <see cref="Box2" /> value representing the rectangular region.</returns>
            public Box2 Bounds { get; }
        }

        /// <summary>
        /// Requests to paint the contents.
        /// </summary>
        /// <param name="p">A structure encapsulating the information required to paint the
        /// contents.</param>
        protected abstract void PaintContents(in PaintParams p);

        /// <summary>
        /// Overrides <see cref="LayerInfo.UpdateLayer(INgsPFLayer, LayerInfo)" />.
        /// </summary>
        [SecuritySafeCritical]
        public override void UpdateLayer(INgsPFLayer layer, LayerInfo previous) {
            // Compute the layer bounds by rounding the desired content bounds.
            // TODO: Take DPI scaling into account
            Box2 bounds = ContentBounds;
            bounds.Max = Vector2.Max(bounds.Max, bounds.Min);
            bounds.Min = new Vector2(MathF.Floor(bounds.Min.X), MathF.Floor(bounds.Min.Y));
            bounds.Max = new Vector2(MathF.Ceiling(bounds.Max.X), MathF.Ceiling(bounds.Max.Y));

            // 1px margin for edge antialiasing
            bounds.Min -= new Vector2(1, 1);
            bounds.Max += new Vector2(1, 1);

            base.Bounds = new Box2(bounds.Min + Origin, bounds.Max + Origin);

            if (previous == null || ShouldUpdate(previous)) {
                // Generate the content image
                var imageSize = new IntVector2((int)bounds.Width, (int)bounds.Height);
                imageSize.X = Math.Min(imageSize.X, 4096);
                imageSize.Y = Math.Min(imageSize.Y, 4096);

                if (imageSize.X > 0 && imageSize.Y > 0) {
                    var bitmap = new Bitmap(imageSize, PixelFormat.SrgbRgba8Premul);

                    using (var painter = bitmap.CreatePainter())
                    using (var _ = painter.Lock()) {
                        painter.Translate(-bounds.Min);
                        PaintContents(new PaintParams(painter, bounds));
                    }

                    var image = bitmap.IntoImage();
                    layer.SetContentsImage(image.NativeImage, new Box2(Vector2.Zero, bounds.Size),
                        ImageWrapMode.Clamp);
                } else {
                    // Empty
                    layer.SetContentsEmpty();
                }
            }

            base.UpdateLayer(layer, previous);
        }
    }
}
