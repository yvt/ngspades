//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.Interop
{
    public interface INativeObject<out T> : IUnknown where T : class, IUnknown
    {
        IntPtr NativeInterfacePtr { get; }
        IntPtr NativeIUnknownPtr { get; }

        T Interface { get; }
    }
}