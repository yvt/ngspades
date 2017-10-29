//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine.Presentation
{
    [Guid("c2a47959-0d5d-46b4-ba83-68c9b69bee56")]
    public interface IPresentationContext : IUnknown
    {
        INodeGroup CreateNodeGroup();
        IWindow CreateWindow();
        ILayer CreateLayer();
    }
}
