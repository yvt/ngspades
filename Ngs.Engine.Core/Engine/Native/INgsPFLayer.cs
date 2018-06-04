//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Utils;
using Ngs.Engine.Presentation;

namespace Ngs.Engine.Native {
    /// <summary>
    /// Represents a layer node, the basic unit of composition.
    /// </summary>
    /// <remarks>
    /// Layer nodes are created from an <see cref="INgsPFContext" /> and
    /// are associated with the context from which they were created.
    /// </remarks>
    [Guid("f6aff079-7478-4b34-9474-6f4960a2818f")]
    public interface INgsPFLayer : IUnknown {
        /// <summary>
        /// Sets the opacity of the layer.
        /// </summary>
        /// <returns>The opacity of the layer in the range <c>[0, 1]</c></returns>
        float Opacity { set; }

        // TODO: contents

        /// <summary>
        /// Sets the transformation matrix of the layer.
        /// </summary>
        /// <returns>The transformation matrix.</returns>
        Matrix4 Transform { set; }

        /// <summary>
        /// Sets the flags specifying the properties of the layer.
        /// </summary>
        /// <returns>The flags specifying the properties of the layer.</returns>
        LayerFlags Flags { set; }

        /// <summary>
        /// Sets the bounding rectangle of the contents or an intermediate raster
        /// image (if <see cref="LayerFlags.FlattenContents" /> is set).
        /// </summary>
        /// <returns>The bounding rectangle of the contents or an intermediate raster image.</returns>
        Box2 Bounds { set; }

        /// <summary>
        /// Sets the child layer(s) of the layer.
        /// </summary>
        /// <returns>
        /// The child layer node (or a group of layer nodes) of the layer, or the <c>null</c> value.
        /// </returns>
        IUnknown Child { set; }

        /// <summary>
        /// Sets the child layer(s) specifying the mask image for this layer.
        /// </summary>
        /// <remarks>
        /// <para>
        /// To enable the mask, this layer must have the `FlattenContents` attribute.
        /// </para>
        /// <para>
        /// Root nodes cannot have a mask enabled.
        /// </para>
        /// </remarks>
        /// <returns>
        /// The child layer node (or a group of layer nodes) of the layer to be used
        /// as the mask image, or the <c>null</c> value indicating no mask image is used.
        /// </returns>
        IUnknown Mask { set; }

        /// <summary>
        /// Sets the contents type to empty.
        /// </summary>
        void SetContentsEmpty();

        /// <summary>
        /// Sets the contents type to solid color, and use the given color.
        /// </summary>
        void SetContentsSolidColor(Rgba color);

        /// <summary>
        /// Sets the contents type to back drop.
        /// </summary>
        void SetContentsBackDrop();

        /// <summary>
        /// Sets the contents type to image, and use the given image and parameters.
        /// </summary>
        void SetContentsImage(IUnknown image, Box2 source, ImageWrapMode wrapMode);

        /// <summary>
        /// Sets the contents type to port, and use the given port object.
        /// </summary>
        void SetContentsPort(IUnknown port);
    }
}