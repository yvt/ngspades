//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using Ngs.Engine;

namespace Ngs.Engine.UI.Widgets {
    /// <summary>
    /// A specialized view for containing zero or more subviews using a layout supplied by the
    /// view's owner.
    /// </summary>
    public class Container : View {
        /// <summary>
        /// Sets or retrieves the layout object used to position the subviews of this view.
        /// </summary>
        /// <returns>The layout object or <c>null</c>.</returns>
        public new Layout Layout {
            get => base.Layout;
            set => base.Layout = value;
        }
    }
}