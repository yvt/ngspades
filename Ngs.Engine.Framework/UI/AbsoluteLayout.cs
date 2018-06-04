//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Collections.Generic;
using System.Numerics;
using Ngs.Engine.UI.Utils;
using Ngs.Engine;

namespace Ngs.Engine.UI {
    /// <summary>
    /// Layouts subviews using absolute distance values relative to the view boundary.
    /// </summary>
    /// <remarks>
    /// Each item (layouted view) in <see cref="AbsoluteLayout" /> is arranged independently using
    /// the constraints specified by the corresponding <see cref="AbsoluteLayout.Item" />.
    /// </remarks>
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

            /// <summary>
            /// Implements <see cref="LayoutItemCollection&lt;T&gt;.OnViewBeingAdded" />.
            /// </summary>
            protected override void OnViewBeingAdded(View view) => layout.AttachSubview(view);

            /// <summary>
            /// Implements <see cref="LayoutItemCollection&lt;T&gt;.OnViewBeingRemoved" />.
            /// </summary>
            protected override void OnViewBeingRemoved(View view) => layout.DetachSubview(view);
        }


        /// <summary>
        /// Represents an item (layouted view) in a <see cref="AbsoluteLayout" />.
        /// </summary>
        public sealed class Item {
            readonly AbsoluteLayout layout;
            float? left, top, right, bottom;
            float? width, height;

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

            /// <summary>
            /// Sets or retrieves the width of the item.
            /// </summary>
            /// <returns>The width of the item.
            /// <c>null</c> indicates that the preferred value provided by the subview should
            /// be used.</returns>
            public float? Width {
                get => width; set {
                    width = value;
                    layout.InvalidateLayout();
                }
            }

            /// <summary>
            /// Sets or retrieves the height of the item.
            /// </summary>
            /// <returns>The height of the item.
            /// <c>null</c> indicates that the preferred value provided by the subview should
            /// be used.</returns>
            public float? Height {
                get => height; set {
                    height = value;
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
            Vector2 minimum = new Vector2(0, 0);
            Vector2 maximum = new Vector2(float.PositiveInfinity, float.PositiveInfinity);

            // Take the mean for preferred size
            Vector2 preferred = new Vector2(0, 0);
            int preferredCountX = 0;
            int preferredCountY = 0;

            foreach (var item in Items) {
                var measurement = item.View.Measurement;
                Vector2 itemMinimum = measurement.MinimumSize;
                Vector2 itemMaximum = measurement.MaximumSize;
                Vector2 itemPreferred = measurement.PreferredSize;

                if (item.Left.HasValue && item.Right.HasValue) {
                    float pad = item.Left.Value + item.Right.Value;

                    if (item.Width is float x) {
                        itemPreferred.X = Math.Clamp(x, itemMinimum.X, itemMaximum.X);
                    }

                    minimum.X = Math.Max(minimum.X, itemMinimum.X + pad);
                    maximum.X = Math.Min(maximum.X, itemMaximum.X + pad);
                    preferred.X += itemPreferred.X + pad;
                    preferredCountX += 1;
                }

                if (item.Top.HasValue && item.Bottom.HasValue) {
                    float pad = item.Top.Value + item.Bottom.Value;

                    if (item.Height is float x) {
                        itemPreferred.Y = Math.Clamp(x, itemMinimum.Y, itemMaximum.Y);
                    }

                    minimum.Y = Math.Max(minimum.Y, itemMinimum.Y + pad);
                    maximum.Y = Math.Min(maximum.Y, itemMaximum.Y + pad);
                    preferred.Y += itemPreferred.Y + pad;
                    preferredCountY += 1;
                }
            }

            if (preferredCountX > 0) {
                preferred.X /= preferredCountX;
            }
            if (preferredCountY > 0) {
                preferred.Y /= preferredCountY;
            }

            return new Measurement()
            {
                MinimumSize = minimum,
                MaximumSize = maximum,
                PreferredSize = preferred,
            };
        }

        /// <summary>
        /// Implements <see cref="Layout.Arrange" />.
        /// </summary>
        public override void Arrange(ArrangeContext context) {
            Vector2 size = View.Bounds.Size;

            foreach (var item in Items) {
                var measurement = item.View.Measurement;
                Vector2 itemMinimum = measurement.MinimumSize;
                Vector2 itemMaximum = measurement.MaximumSize;
                Vector2 itemPreferred = measurement.PreferredSize;

                if (item.Width is float x) {
                    itemPreferred.X = Math.Clamp(x, itemMinimum.X, itemMaximum.X);
                }
                if (item.Height is float y) {
                    itemPreferred.Y = Math.Clamp(y, itemMinimum.Y, itemMaximum.Y);
                }

                Vector2 itemSize = itemPreferred;
                Vector2 itemPosition;

                if (item.Left.HasValue) {
                    if (item.Right.HasValue) {
                        itemSize.X = size.X - item.Left.Value - item.Right.Value;
                    }
                    itemPosition.X = item.Left.Value;
                } else if (item.Right.HasValue) {
                    itemPosition.X = size.X - itemSize.X - item.Right.Value;
                } else {
                    itemPosition.X = (size.X - itemSize.X) * 0.5f;
                }

                if (item.Top.HasValue) {
                    if (item.Bottom.HasValue) {
                        itemSize.Y = size.Y - item.Top.Value - item.Bottom.Value;
                    }
                    itemPosition.Y = item.Top.Value;
                } else if (item.Bottom.HasValue) {
                    itemPosition.Y = size.Y - itemSize.Y - item.Bottom.Value;
                } else {
                    itemPosition.Y = (size.Y - itemSize.Y) * 0.5f;
                }

                context.ArrangeSubview(item.View, new Box2(itemPosition, itemPosition + itemSize));
            }
        }
    }
}
