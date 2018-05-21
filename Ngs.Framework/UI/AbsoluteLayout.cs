//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Collections.Generic;
using System.Numerics;
using Ngs.UI.Utils;

namespace Ngs.UI {
    public sealed class AbsoluteLayout : Layout {
        /// <summary>
        /// Represents a collection of <see cref="Item" /> objects in a <see cref="AbsoluteLayout" />.
        /// </summary>
        public sealed class ItemCollection : LayoutItemCollection<Item> {
            private readonly AbsoluteLayout layout;

            internal ItemCollection(AbsoluteLayout layout) => this.layout = layout;

            /// <summary>
            /// Implements <see cref="LayoutItemCollection&lt;T&gt;.CreateItem" />.
            /// </summary>
            protected override Item CreateItem(View view) => new Item(layout, view);

            /// <summary>
            /// Implements <see cref="LayoutItemCollection&lt;T&gt;.OnUpdate" />.
            /// </summary>
            protected override void OnUpdate() => layout.InvalidateLayout();
        }

        /// <summary>
        /// Represents an item (layouted view) in a <see cref="AbsoluteLayout" />.
        /// </summary>
        public sealed class Item {
            private readonly AbsoluteLayout layout;
            private float? left;
            private float? top;
            private float? right;
            private float? bottom;

            internal Item(AbsoluteLayout layout, View view) {
                this.layout = layout;
                View = view;
            }

            /// <summary>
            /// Retrieves the view associated with this item.
            /// </summary>
            /// <returns>The view associated with this item.</returns>
            public View View { get; }

            /// <summary>
            /// Sets or retrieves the distance of this item from the left edge of the container box.
            /// </summary>
            /// <returns>The distance, measured in device independent pixels.
            /// <c>null</c> indicates that the distance is not restricted.</returns>
            public float? Left {
                get => left; set {
                    left = value;
                    layout.InvalidateLayout();
                }
            }

            /// <summary>
            /// /// Sets or retrieves the distance of this item from the top edge of the container box.
            /// </summary>
            /// <returns>The distance, measured in device independent pixels.
            /// <c>null</c> indicates that the distance is not restricted.</returns>
            public float? Top {
                get => top; set {
                    top = value;
                    layout.InvalidateLayout();
                }
            }

            /// <summary>
            /// Sets or retrieves the distance of this item from the right edge of the container box.
            /// </summary>
            /// <returns>The distance, measured in device independent pixels.
            /// <c>null</c> indicates that the distance is not restricted.</returns>
            public float? Right {
                get => right; set {
                    right = value;
                    layout.InvalidateLayout();
                }
            }

            /// <summary>
            /// Sets or retrieves the distance of this item from the bottom edge of the container box.
            /// </summary>
            /// <returns>The distance, measured in device independent pixels.
            /// <c>null</c> indicates that the distance is not restricted.</returns>
            public float? Bottom {
                get => bottom; set {
                    bottom = value;
                    layout.InvalidateLayout();
                }
            }
        }

        /// <summary>
        /// Initializes a new instance of <see cref="AbsoluteLayout" />.
        /// </summary>
        public AbsoluteLayout() {
            Items = new ItemCollection(this);
        }

        /// <summary>
        /// Retrieves the collection of items in this layout object.
        /// </summary>
        /// <returns>The collection of items in this layout object.</returns>
        public ItemCollection Items { get; }

        /// <summary>
        /// Implements <see cref="Layout.Subviews" />.
        /// </summary>
        public override IEnumerable<View> Subviews { get => Items.Subviews; }

        /// <summary>
        /// Implements <see cref="Layout.Measure" />.
        /// </summary>
        public override Measurement Measure() {
            throw new NotImplementedException(); // TODO: Implement `AbsoluteLayout`
        }

        /// <summary>
        /// Implements <see cref="Layout.Arrange" />.
        /// </summary>
        public override void Arrange(ArrangeContext context) {
            throw new NotImplementedException(); // TODO: Implement `AbsoluteLayout`
        }
    }
}
