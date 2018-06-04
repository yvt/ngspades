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
    /// Represents a single font face in a font file.
    /// </summary>
    /// <remarks>
    /// <para>This class is a wrapper of <see cref="INgsPFFontFace" />.</para>
    /// </remarks>
    public struct FontFace {
        private INgsPFFontFace nativeObject;

        internal FontFace(INgsPFFontFace nativeObject) {
            this.nativeObject = nativeObject;
        }

        internal INgsPFFontFace NativeFontFace {
            [SecurityCritical]
            get => nativeObject;
        }
    }
}
