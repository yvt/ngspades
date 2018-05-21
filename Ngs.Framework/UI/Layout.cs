//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Collections.Generic;
using System.Numerics;
using Ngs.Utils;

namespace Ngs.UI {
    /// <summary>
    /// Represents a layout object, which describes how their subviews should be arranged within
    /// a view.
    /// </summary>
    public abstract class Layout {
        internal View view;

        /// <summary>
        /// Retrieves the view associated with this layout.
        /// </summary>
        /// <returns>The view associated with this layout or <c>null</c>.</returns>
        public View View { get => view; }

        /// <summary>
        /// Called by a derived class when a subview was added to this layout.
        /// </summary>
        /// <exception cref="InvalidOperationException"><paramref name="view" />
        /// already has been added to another layout.</exception>
        /// <param name="view">The view to be added as a subview.</param>
        protected void AttachSubview(View view) {
            if (view.superviewLayout != null) {
                throw new InvalidOperationException("The view already already has been added to another layout.");
            }
            view.superviewLayout = this;
        }

        /// <summary>
        /// Called by a derived class when a subview was removed from this layout.
        /// </summary>
        /// <exception cref="InvalidOperationException"><paramref name="view" />
        /// is not associated with this layout.</exception>
        /// <param name="view">The view to be removed.</param>
        protected void DetachSubview(View view) {
            if (view.superviewLayout != this) {
                throw new InvalidOperationException("The view is not associated with this layout.");
            }
            view.superviewLayout = null;
        }

        /// <summary>
        /// Retrieves the list of subviews in this layout.
        /// </summary>
        /// <returns>The list of subviews in this layout.</returns>
        public abstract IEnumerable<View> Subviews { get; }

        /// <summary>
        /// Called by a derived class when layout properties have changed.
        /// </summary>
        /// <remarks>
        /// Calling this provokes the execution of the layout algorithm on the associated view.
        /// The <see cref="Measure" /> and <see cref="Arrange" /> methods will be called on this
        /// layout in the respective order, provided that this layout is included in some view tree
        /// that is actually visible.
        /// </remarks>
        protected void InvalidateLayout() {
            if (View is View view) {
                view.SetNeedsMeasure();
                view.SetNeedsArrange();
            }
        }

        /// <summary>
        /// Provides a method to arrange subviews in a layout to be called by an implementation of
        /// <see cref="Arrange" />.
        /// </summary>
        public readonly ref struct ArrangeContext {
            private readonly Layout layout;

            internal ArrangeContext(Layout layout) => this.layout = layout;

            /// <summary>
            /// Sets the position of a view.
            /// </summary>
            /// <param name="view">The view to arrange.</param>
            /// <param name="bounds">The new bounding rectangle of the view.</param>
            /// <exception name="InvalidOperationException">The specified view is not a subview of
            /// the current layout.</exception>
            public void ArrangeSubview(View view, Box2 bounds) {
                if (view.superviewLayout != layout) {
                    throw new InvalidOperationException("The view is not a subview of the layout.");
                }

                view.Bounds = bounds;
            }
        }

        /// <summary>
        /// Performs the measurement step of the layouting algorithm.
        /// </summary>
        public abstract Measurement Measure();

        /// <summary>
        /// Performs the arrangement step of the layouting algorithm.
        /// </summary>
        /// <remarks>
        /// The implementation must call <see cref="ArrangeContext.ArrangeSubview" /> on all
        /// subviews to arrange them.
        /// </remarks>
        public abstract void Arrange(ArrangeContext context);

        internal void ArrangeInternal() {
            Arrange(new ArrangeContext(this));
        }
    }
}
