//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine.Native {
    /// <summary>
    /// Provides an access to the presentation context.
    /// </summary>
    /// <remarks>
    /// <para>Nodes (windows, layers, and node groups) are created from a specific presentation
    /// context. Once created, they only can be used within the same context.</para>
    /// <para>Every property of a node is protected by a single mutex maintained by the node
    /// which the node belongs to. See <see cref="Lock" /> for how to minimize the runtime overhead
    /// incurred by the mutual exclusion.</para>
    /// </remarks>
    [Guid("c2a47959-0d5d-46b4-ba83-68c9b69bee56")]
    public interface INgsPFContext : IUnknown {
        /// <summary>
        /// Creates a node group.
        /// </summary>
        /// <returns>An <see cref="INgsPFNodeGroup" /> object refering to the created node group.</returns>
        INgsPFNodeGroup CreateNodeGroup();

        /// <summary>
        /// Creates a window node.
        /// </summary>
        /// <returns>An <see cref="INgsPFWindow" /> object refering to the created window node.</returns>
        INgsPFWindow CreateWindow();

        /// <summary>
        /// Creates a layer node.
        /// </summary>
        /// <returns>An <see cref="INgsPFLayer" /> object refering to the created layer node.</returns>
        INgsPFLayer CreateLayer();

        /// <summary>
        /// Acquires a lock on the context state for the current thread.
        /// Fails thread if another thread currently holds a lock.
        /// </summary>
        /// <remarks>
        /// <para>Every property of a node is protected by a single mutex maintained by its parent
        /// presentation context. By default, a lock on this mutex is acquired every time you
        /// access a property. Since every lock operation incurs a moderate performance cost,
        /// you can alternatively choose to explicitly acquire a lock for an extended duration
        /// by using this method.</para>
        /// <para>The acquired lock will be linked to the current thread, and lasts until
        /// <see cref="Unlock" /> is called from the same thread.</para>
        /// </remarks>
        void Lock();

        /// <summary>
        /// Releases a lock on the context state acquired by <see cref="Lock" />.
        /// </summary>
        /// <remarks>
        /// The calls to this method must be matched by the same number of calls
        /// to <see cref="Lock" />.
        /// A failure to call this method for sufficient times might result in a dead-lock.
        /// </remarks>
        void Unlock();

        /// <summary>
        /// Captures the current state of presentation nodes and submit it for
        /// presentation.
        /// </summary>
        /// <remarks>
        /// If you have a lock on the context state (acquired by <see cref="Lock" />), you must
        /// release it first.
        /// </remarks>
        void CommitFrame();
    }
}