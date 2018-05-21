//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using Ngs.Utils;

namespace Ngs.UI {
    /// <summary>
    /// A specialized view use to contain zero or more subviews.
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