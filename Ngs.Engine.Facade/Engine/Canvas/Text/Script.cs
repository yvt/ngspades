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
    /// Specifies a script used during a text layouting operation.
    /// </summary>
    public struct Script {
        private IUnknown nativeObject;

        internal Script(IUnknown nativeObject) {
            this.nativeObject = nativeObject;
        }

        /// <summary>
        /// Retrieves a script object corresponding to a given ISO15924 code.
        /// </summary>
        /// <param name="iso15924">The ISO15924 code.</param>
        /// <returns>A script object.</returns>
        public static Script FromIso15924(string iso15924) =>
            new Script(EngineInstance.NativeEngine.FontFactory.GetScriptFromIso15924(iso15924));

        internal IUnknown NativeObject {
            [SecurityCritical]
            get => nativeObject;
        }
    }
}
