//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.Engine.Presentation
{
    /// <summary>
    /// Specifies properties of a layer.
    /// </summary>
    [Flags]
    public enum LayerFlags
    {
        /// <summary>
        /// <para>
        /// Instructs to rasterize the contents of the layer.
        /// </para>
        /// <para>
        /// When this flag is specified, the contents (including its children) of the layer is
        /// rendered as a raster image and then composied to the parent image. This flag is required
        /// to enable the following composition features: filters and layer masks.
        /// </para>
        /// </summary>
        FlattenContents = 1 << 0,
    }
}
