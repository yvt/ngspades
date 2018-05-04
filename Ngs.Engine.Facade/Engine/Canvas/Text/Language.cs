//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Security;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Utils;
using Ngs.Engine.Native;

namespace Ngs.Engine.Canvas.Text {
    /// <summary>
    /// Specifies a language used during a text layouting operation.
    /// </summary>
    public struct Language {
        private IUnknown nativeObject;

        internal Language(IUnknown nativeObject) {
            this.nativeObject = nativeObject;
        }

        /// <summary>
        /// Retrieves a language object corresponding to a given ISO639 code.
        /// </summary>
        /// <param name="iso639">The ISO639 code.</param>
        /// <returns>A language object.</returns>
        public static Language FromIso639(string iso639) =>
            new Language(EngineInstance.NativeEngine.FontFactory.GetLanguageFromIso639(iso639));

        internal IUnknown NativeObject {
            [SecurityCritical]
            get => nativeObject;
        }
    }
}
