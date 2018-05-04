//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Utils;

namespace Ngs.Engine.Native {
    /// <summary>
    /// Provides factory methods for the font subsystem.
    /// </summary>
    [Guid("907c6f37-edf5-4a23-9f28-1077d602789f")]
    public interface INgsPFFontFactory : IUnknown {
        /// <summary>
        /// Creates a font object.
        /// </summary>
        /// <param name="bytes">A pointer to a TrueType or OpenType binary blob.</param>
        /// <param name="length">The size of the binary blob.</param>
        /// <returns>A newly created font object.</returns>
        [System.Security.SecurityCritical]
        INgsPFFont CreateFont(IntPtr bytes, int length);

        /// <summary>
        /// Creates a font config object.
        /// </summary>
        /// <returns>A newly created font config object.</returns>
        INgsPFFontConfig CreateFontConfig();

        /// <summary>
        /// Creates a paragraph style object.
        /// </summary>
        /// <returns>A newly created paragraph style object.</returns>
        INgsPFParagraphStyle CreateParagraphStyle();

        /// <summary>
        /// Creates a character style object.
        /// </summary>
        /// <returns>A newly created character style object.</returns>
        INgsPFCharStyle CreateCharStyle();

        /// <summary>
        /// Retrieves a script object corresponding to a given ISO15924 code.
        /// </summary>
        /// <param name="iso15924">The ISO15924 code.</param>
        /// <returns>A script object.</returns>
        IUnknown GetScriptFromIso15924(string iso15924);

        /// <summary>
        /// Retrieves a language object corresponding to a given ISO639 code.
        /// </summary>
        /// <param name="iso639">The ISO639 code.</param>
        /// <returns>A language object.</returns>
        IUnknown GetLanguageFromIso639(string iso639);
    }
}