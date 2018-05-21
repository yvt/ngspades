//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Collections.Generic;

namespace Ngs.UI {
    /// <summary>
    /// A layout object to be used to hold the root view of a window.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The primary goal of using this object is to prevent root views from being inserted to other
    /// layouts. The application is not supposed to see this object (otherwise, it would be possible
    /// to assign this object to <c>View.Layout</c>).
    /// </para><para>
    /// Accordingly, this layout object will never be used for actual layouting in any ways.
    /// </para>
    /// </remarks>
    internal sealed class WindowContentsLayout : Layout {
        private View rootView;

        public View ContentsView {
            get => this.rootView;
            set {
                if (this.rootView != null) {
                    DetachSubview(rootView);
                    this.rootView = null;
                }
                if (value != null) {
                    AttachSubview(value);
                    this.rootView = value;
                }
            }
        }

        public override IEnumerable<View> Subviews => throw new NotSupportedException();
        public override void Arrange(ArrangeContext context) => throw new NotSupportedException();
        public override Measurement Measure() => throw new NotSupportedException();
    }
}
