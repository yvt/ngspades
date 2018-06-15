//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using Ngs.Engine;
using Ngs.Engine.Canvas;
using Ngs.Engine.Canvas.Text;

namespace Ngs.Engine.UI.Theming {
    /// <summary>
    /// Maintains globally shared instances of <see cref="FontConfig" /> and
    /// <see cref="ParagraphStyle" />. Broadcasts an update event when they are updated.
    /// </summary>
    /// <remarks>
    /// This class is thread safe.
    /// </remarks>
    public sealed class FontManager {
        /// <summary>
        /// Retrieves the default instance of <see cref="FontManager" />.
        /// </summary>
        /// <returns>The default instance of <see cref="FontManager" />.</returns>
        public static FontManager Default { get; } = new FontManager();

        /// <summary>
        /// Initializes a new instance of this class.
        /// </summary>
        public FontManager() {
            defaultParagraphStyle.CharacterStyle.FontSize = 14;
        }

        FontConfig fontConfig = new FontConfig();

        /// <summary>
        /// Retrieves the managed <see cref="FontConfig" /> instance.
        /// </summary>
        /// <remarks>
        /// <para>The <see cref="FontConfigUpdated" /> event is raised whenever the
        /// <see cref="ReadOnlyFontConfig" /> instance represented by this property is updated, or
        /// it is replaced with a new instance.</para>
        /// <para>Do not attempt to modify the returned <see cref="ReadOnlyFontConfig" /> directly.
        /// </para>
        /// </remarks>
        public ReadOnlyFontConfig FontConfig => fontConfig;

        /// <summary>
        /// Occurs when <see cref="FontConfig" /> was updated.
        /// </summary>
        public event EventHandler FontConfigUpdated;

        /// <summary>
        /// Adds a font face into the font search list.
        /// </summary>
        /// <param name="fontFace">The font face to be added.</param>
        /// <param name="fontFamily">The font family name associated with the font face.</param>
        /// <param name="fontStyle">The font style associated with the font face. Mustn't be <c>Inherited</c>.</param>
        /// <param name="weight">The font weight associated with the font face. Must be in range <c>[100, 1000]</c>.</param>
        public void AddFontFace(FontFace fontFace, string fontFamily, FontStyle fontStyle, int weight) {
            this.fontConfig.AddFontFace(fontFace, fontFamily, fontStyle, weight);
            FontConfigUpdated?.Invoke(this, EventArgs.Empty);
        }

        private readonly ParagraphStyle defaultParagraphStyle = new ParagraphStyle();

        /// <summary>
        /// Occurs when <see cref="DefaultParagraphStyle" /> was updated.
        /// </summary>
        public event EventHandler DefaultParagraphStyleUpdated;

        /// <summary>
        /// Retrieves the default paragraph style.
        /// </summary>
        /// <returns>The default paragraphy style.</returns>
        public ReadOnlyParagraphStyle DefaultParagraphStyle => defaultParagraphStyle.AsReadOnly();

        /// <summary>
        /// Assigns a new default paragraph style.
        /// </summary>
        /// <param name="newStyle">The paragraph style to assign.</param>
        public void SetDefaultParagraphStyle(ReadOnlyParagraphStyle newStyle) {
            if (newStyle == null) {
                throw new ArgumentNullException(nameof(newStyle));
            }
            defaultParagraphStyle.CopyFrom(newStyle);
            DefaultParagraphStyleUpdated?.Invoke(this, EventArgs.Empty);
        }
    }
}