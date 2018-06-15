//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using Ngs.Engine.Canvas.Text;
using Ngs.Engine;
using Ngs.Engine.UI.Theming;

namespace Ngs.Engine.UI.Widgets {
    /// <summary>
    /// Represents a label widget.
    /// </summary>
    public class Label : View {
        FontManager fontManager = FontManager.Default;
        ReadOnlyFontConfig fontConfig;
        ReadOnlyParagraphStyle paragraphStyle;
        string text = "";
        float? width;
        Rgba textColor = Rgba.White;

        bool mounted;
        bool hasFontConfigUpdatedHandlerRegistered;
        bool hasParagraphStyleUpdatedHandlerRegistered;

        void RegisterFontManagerUpdateHandlers(bool unregisterAll = false) {
            var fontManager = this.fontManager;

            if (!mounted) {
                unregisterAll = true;
            }

            bool needsFontConfigUpdateHandler = !unregisterAll && fontConfig == null;
            bool needsParagraphStyleUpdateHandler = !unregisterAll && paragraphStyle == null;

            if (needsFontConfigUpdateHandler != hasFontConfigUpdatedHandlerRegistered) {
                if (needsFontConfigUpdateHandler) {
                    fontManager.FontConfigUpdated += HandleFontManagerUpdate;
                } else {
                    fontManager.FontConfigUpdated -= HandleFontManagerUpdate;
                }
                hasFontConfigUpdatedHandlerRegistered = needsFontConfigUpdateHandler;
            }

            if (needsParagraphStyleUpdateHandler != hasParagraphStyleUpdatedHandlerRegistered) {
                if (needsParagraphStyleUpdateHandler) {
                    fontManager.DefaultParagraphStyleUpdated += HandleFontManagerUpdate;
                } else {
                    fontManager.DefaultParagraphStyleUpdated -= HandleFontManagerUpdate;
                }
                hasParagraphStyleUpdatedHandlerRegistered = needsParagraphStyleUpdateHandler;
            }
        }

        void HandleFontManagerUpdate(object sender, EventArgs e) {
            InvalidateTextLayout();
        }

        /// <summary>
        /// Sets or retrieves the font manager object used as a source of the default font config
        /// and paragraph style objects when they are not specified by the
        /// <see cref="FontConfig" /> and <see cref="ParagraphStyle" /> properties.
        /// </summary>
        /// <returns>The font manager object that provides default values.</returns>
        public FontManager FontManager {
            get => fontManager; set {
                if (value == fontManager) {
                    return;
                }
                if (value == null) {
                    throw new ArgumentNullException();
                }
                RegisterFontManagerUpdateHandlers(true);
                fontManager = value;
                RegisterFontManagerUpdateHandlers();
            }
        }

        /// <summary>
        /// Sets or retrieves the font config object used to layout the text.
        /// </summary>
        /// <returns>The font config object used to layout the text. <c>null</c> indicates that the
        /// font config is provided by <see cref="FontManager" />.</returns>
        public ReadOnlyFontConfig FontConfig {
            get => fontConfig; set {
                fontConfig = value;
                RegisterFontManagerUpdateHandlers();
                InvalidateTextLayout();
            }
        }

        /// <summary>
        /// Sets or retrieves the paragraph style object used to layout the text.
        /// </summary>
        /// <returns>The paragraph style object used to layout the text. <c>null</c> indicates that
        /// the font config is provided by <see cref="FontManager" />.</returns>
        public ReadOnlyParagraphStyle ParagraphStyle {
            get => paragraphStyle; set {
                paragraphStyle = value;
                RegisterFontManagerUpdateHandlers();
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
                var fontConfig = this.fontConfig ?? this.fontManager.FontConfig;
                var paragraphStyle = this.paragraphStyle ?? this.fontManager.DefaultParagraphStyle;

                if (this.width is float width) {
                    textLayout = fontConfig.LayoutString(text, paragraphStyle, width);
                } else {
                    textLayout = fontConfig.LayoutString(text, paragraphStyle);
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

        /// <summary>
        /// Overrides <see cref="View.OnMounted" />.
        /// </summary>
        protected virtual void OnMounted(EventArgs e) {
            mounted = true;
            RegisterFontManagerUpdateHandlers();
            InvalidateTextLayout();
        }

        /// <summary>
        /// Overrides <see cref="View.OnUnmounted" />.
        /// </summary>
        protected virtual void OnUnmounted(EventArgs e) {
            mounted = false;
            RegisterFontManagerUpdateHandlers(true);
        }

    }
}