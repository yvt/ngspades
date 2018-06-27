//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Engine.Presentation;

namespace Ngs.Engine.Native {
    /// <summary>
    /// Represents a window node.
    /// </summary>
    /// <remarks>
    /// Window nodes are created from an <see cref="INgsPFContext" /> and
    /// are associated with the context from which they were created.
    /// </remarks>
    [Guid("1fd3658b-e4ac-49bb-9609-a0e578022cbc")]
    public interface INgsPFWindow : IUnknown {
        /// <summary>
        /// Sets the flags specifying the properties of the window.
        /// </summary>
        /// <returns>The flags specifying the properties of the window.</returns>
        WindowFlags Flags { set; }

        /// <summary>
        /// Sets the size of the window.
        /// </summary>
        /// <returns>The size of the window measured in device independent pixels.</returns>
        Vector2 Size { set; }

        /// <summary>
        /// Sets the maximum size of the window.
        /// </summary>
        /// <returns>The maximum size of the window measured in device independent pixels.</returns>
        Vector2 MaximumSize { set; }

        /// <summary>
        /// Sets the minimum size of the window.
        /// </summary>
        /// <returns>The minimum size of the window measured in device independent pixels.</returns>
        Vector2 MinimumSize { set; }

        /// <summary>
        /// Sets the child layer(s) of the window.
        /// </summary>
        /// <returns>
        /// The child layer node (or a group of layer nodes) of the window, or the <c>null</c> value.
        /// </returns>
        IUnknown Child { set; }

        /// <summary>
        /// Sets the title text of the window.
        /// </summary>
        /// <returns>The title text of the window.</returns>
        string Title { set; }

        /// <summary>
        /// Sets the <see cref="INgsPFWindowListener" /> object that receives and handles window events.
        /// </summary>
        /// <returns>
        /// the <see cref="INgsPFWindowListener" /> object that receives and handles window events, or <c>null</c>.
        /// </returns>
        INgsPFWindowListener Listener { set; }
    }
}