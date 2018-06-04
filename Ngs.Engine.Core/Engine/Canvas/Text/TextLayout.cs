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
    /// A layouted text.
    /// </summary>
    /// <remarks>
    /// <para>This class is a wrapper of <see cref="INgsPFTextLayout" />.</para>
    /// </remarks>
    public class TextLayout {
        private INgsPFTextLayout nativeObject;

        internal TextLayout(INgsPFTextLayout nativeObject) {
            this.nativeObject = nativeObject;
        }

        internal INgsPFTextLayout NativeTextLayout {
            [SecurityCritical]
            get => nativeObject;
        }

        /// <summary>
        /// Computes and retrieves the visual bounding rectangle of the layouted text.
        /// </summary>
        /// <returns>The visual bounding rectangle of the layouted text.</returns>
        public Box2 VisualBounds { get => nativeObject.VisualBounds; }
    }
}