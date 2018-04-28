//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Utils;

namespace Ngs.Engine {
    /// <summary>
    /// Provides an entry point to access the game engine's features.
    /// </summary>
    [Guid("d8104475-a3b1-4221-9133-935c157f3f92")]
    public interface IEngine : IUnknown {
        /// <summary>
        /// Creates a workspace.
        /// </summary>
        /// <param name="listener">An application-provided listener object that will respond to
        /// events from the newly created workspace.</param>
        /// <returns>A new <see cref="Ngs.Engine.Presentation.IWorkspace" />.</returns>
        Presentation.IWorkspace CreateWorkspace(Presentation.IWorkspaceListener listener);

        /// <summary>
        /// Creates a bitmap.
        /// </summary>
        /// <param name="size">The size of the bitmap, measured in pixels.</param>
        /// <param name="format">The pixel representation format of the bitmap.</param>
        /// <returns>An newly created empty bitmap.</returns>
        Canvas.IBitmap CreateBitmap(IntVector2 size, Canvas.PixelFormat format);
    }
}