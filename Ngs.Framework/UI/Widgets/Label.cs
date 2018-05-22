//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using Ngs.Engine.Canvas.Text;
using Ngs.Utils;

namespace Ngs.UI.Widgets {
    /// <summary>
    /// Represents a label widget.
    /// </summary>
    public class Label : View {
        FontConfig fontConfig;
        ParagraphStyle paragraphStyle = new ParagraphStyle();
        string text;
        float? width;
        Rgba textColor = Rgba.White;

        /// <summary>
        /// Sets or retrieves the font config object used to layout the text.
        /// </summary>
        /// <returns>The font config object used to layout the text.</returns>
        public FontConfig FontConfig {
            get => fontConfig; set {
                // FIXME: `FontConfig` is not immutable
                fontConfig = value;
                InvalidateTextLayout();
            }
        }

        /// <summary>
        /// Sets or retrieves the paragraph style object used to layout the text.
        /// </summary>
        /// <returns>The paragraph style object used to layout the text.</returns>
        public ParagraphStyle ParagraphStyle {
            get => paragraphStyle; set {
                // FIXME: `ParagraphStyle` is not immutable
                paragraphStyle = value ?? new ParagraphStyle();
                InvalidateTextLayout();
            }
        }

        /// <summary>
        /// Sets or retrieves the text displayed on this view.
        /// </summary>
        /// <returns>The text displayed on this view.</returns>
        public string Text {
            get => text; set {
                value = value ?? "";
                if (text == value) {
                    return;
                }
                text = value;
                InvalidateTextLayout();
            }
        }

        /// <summary>
        /// Sets or retrieves the width of this label.
        /// </summary>
        /// <returns>The width of this label. <c>null</c> indicates that the width is calculated
        /// automatically to fit the text.</returns>
        public float? Width {
            get => width;
            set {
                if (width == value) {
                    return;
                }
                width = value;
                InvalidateTextLayout();
            }
        }

        /// <summary>
        /// Sets or retrieves the text color.
        /// </summary>
        /// <returns>The text color.</returns>
        public Rgba TextColor {
            get => textColor; set {
                textColor = value;
                SetNeedsRender();
            }
        }

        TextLayout textLayout;

        void InvalidateTextLayout() {
            textLayout = null;
            InvalidateInherentLayoutProps();
            SetNeedsRender();
        }

        TextLayout TextLayout {
            get {
                if (textLayout == null && fontConfig != null) {
                    if (this.width is float width) {
                        textLayout = fontConfig.LayoutString(text, paragraphStyle, width);
                    } else {
                        textLayout = fontConfig.LayoutString(text, paragraphStyle);
                    }
                }
                return textLayout;
            }
        }

        Vector2 TextSize {
            // TODO: Use the logical size (a.k.a. selection bounds)
            get {
                if (TextLayout == null) {
                    return Vector2.Zero;
                }

                var bounds = TextLayout.VisualBounds;
                return new Vector2(width ?? bounds.Max.X, -bounds.Min.Y);
            }
        }

        /// <summary>
        /// Overrides <see cref="View.MinimumSize" />.
        /// </summary>
        public override Vector2 MinimumSize { get => TextSize; }
        /// <summary>
        /// Overrides <see cref="View.MaximumSize" />.
        /// </summary>
        public override Vector2 MaximumSize { get => TextSize; }
        /// <summary>
        /// Overrides <see cref="View.PreferredSize" />.
        /// </summary>
        public override Vector2? PreferredSize { get => TextSize; }

        /// <summary>
        /// Overrides <see cref="View.RenderContents" />.
        /// </summary>
        protected override void RenderContents(RenderContext context) {
            var textLayout = this.TextLayout;
            if (textLayout == null) {
                return;
            }

            context.EmitLayer(new TextLayoutLayerInfo(textLayout, textColor)
            {
                Origin = new Vector2(0, Bounds.Height),
            });
        }
    }
}