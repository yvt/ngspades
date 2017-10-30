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
    /// <summary>
    /// Provides an access to the presentation context.
    /// </summary>
    /// <remarks>
    /// Nodes (windows, layers, and node groups) are created from a specific presentation context.
    /// Once created, they only can be used within the same context.
    /// </remarks>
    [Guid("c2a47959-0d5d-46b4-ba83-68c9b69bee56")]
    public interface IPresentationContext : IUnknown
    {
        /// <summary>
        /// Creates a node group.
        /// </summary>
        /// <returns>An <see cref="INodeGroup" /> object refering to the created node group.</returns>
        INodeGroup CreateNodeGroup();

        /// <summary>
        /// Creates a window node.
        /// </summary>
        /// <returns>An <see cref="IWindow" /> object refering to the created window node.</returns>
        IWindow CreateWindow();

        /// <summary>
        /// Creates a layer node.
        /// </summary>
        /// <returns>An <see cref="ILayer" /> object refering to the created layer node.</returns>
        ILayer CreateLayer();
    }
}
