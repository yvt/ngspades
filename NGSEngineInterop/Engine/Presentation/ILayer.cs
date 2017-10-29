//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Utils;

namespace Ngs.Engine.Presentation
{
	[Guid("f6aff079-7478-4b34-9474-6f4960a2818f")]
	public interface ILayer : IUnknown
	{
        /// Sets the opacity of the layer.
        float Opacity { set; }

        // TODO: contents

        /// Sets the bounding rectangle of the contents or an intermediate raster
        /// image (if `FlattenContents` is set).
        Box2 Bounds { set; }

        /// Sets the child layer(s) of the layer.
        IUnknown Child { set; }

        /// Sets the child layer(s) specifying the mask image for this layer.
        ///
        /// To enable the mask, this layer must have the `FlattenContents` attribute.
        ///
        /// Root nodes cannot have a mask enabled.
        IUnknown Mask { set; }
	}
}
