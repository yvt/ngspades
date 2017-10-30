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
    /// Represents a node group.
    /// </summary>
    /// <remarks>
    /// Node groups are created from an <see cref="IPresentationContext" /> and
    /// are associated with the context from which they were created.
    /// </remarks>
    [Guid("77c92e07-8698-41d5-af4c-7f4fb7a4f328")]
    public interface INodeGroup : IUnknown
    {
        /// <summary>
        /// Insert a node to the node group.
        /// </summary>
        /// <remarks>
        /// The node group must not have been attached as a child yet.
        /// Attaching a node group will turn it into an immutable state.
        /// </remarks>
        void Insert(IUnknown node);
    }
}
