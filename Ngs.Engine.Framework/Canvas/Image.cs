//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Security;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Engine;

namespace Ngs.Engine.Canvas {
    /// <summary>
    /// Represents an immutable bitmap image.
    /// </summary>
    public class Image {
        private IUnknown nativeImage;

        internal Image(IUnknown nativeImage) {
            this.nativeImage = nativeImage;
        }

        public IUnknown NativeImage {
            [SecurityCritical]
            get => nativeImage;
        }
    }
}