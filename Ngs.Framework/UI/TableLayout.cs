//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Collections;
using System.Collections.Generic;
using System.Numerics;
using System.ComponentModel;
using Ngs.UI.Utils;

namespace Ngs.UI {
    public sealed class TableLayout : Layout {
        /// <summary>
        /// Represents a collection of <see cref="Column" /> objects in a <see cref="TableLayout" />.
        /// </summary>
        public sealed class ColumnCollection : ResizableList<Column> {
            private readonly TableLayout layout;
            internal ColumnCollection(TableLayout layout) => this.layout = layout;

            /// <summary>
            /// Implements <see cref="ResizableList&lt;T&gt;.CreateItem" />.
            /// </summary>
            protected override Column CreateItem() => new Column(layout);

            /// <summary>
            /// Implements <see cref="ResizableList&lt;T&gt;.OnUpdate" />.
            /// </summary>
            protected override void OnUpdate() => layout.InvalidateLayout();
        }

        /// <summary>
        /// Represents a column in a <see cref="TableLayout" />.
        /// </summary>
        public sealed class Column {
            private readonly TableLayout layout;
            private float? width;

            internal Column(TableLayout layout) {
                this.layout = layout;
            }

            /// <summary>
            /// Sets or retrieves the preferred width of this column.
            /// </summary>
            /// <returns>The preferred width of this column.</returns>
            public float? Width {
                get => width;
                set {
                    width = value;
                    layout.InvalidateLayout();
                }
            }
        }

        /// <summary>
        /// Represents a collection of <see cref="Row" /> objects in a <see cref="TableLayout" />.
        /// </summary>
        public sealed class RowCollection : ResizableList<Row> {
            private readonly TableLayout layout;
            internal RowCollection(TableLayout layout) => this.layout = layout;

            /// <summary>
            /// Implements <see cref="ResizableList&lt;T&gt;.CreateItem" />.
            /// </summary>
            protected override Row CreateItem() => new Row(layout);

            /// <summary>
            /// Implements <see cref="ResizableList&lt;T&gt;.OnUpdate" />.
            /// </summary>
            protected override void OnUpdate() => layout.InvalidateLayout();
        }

        /// <summary>
        /// Represents a row in a <see cref="TableLayout" />.
        /// </summary>
        public sealed class Row {
            private readonly TableLayout layout;
            private float? height;

            internal Row(TableLayout layout) {
                this.layout = layout;
            }

            /// <summary>
            /// Sets or retrieves the preferred height of this row.
            /// </summary>
            /// <returns>The preferred height of this row.</returns>
            public float? Height {
                get => height;
                set {
                    height = value;
                    layout.InvalidateLayout();
                }
            }
        }

        /// <summary>
        /// Represents a collection of <see cref="Item" /> objects in a <see cref="TableLayout" />.
        /// </summary>
        public sealed class ItemCollection : LayoutItemCollection<Item> {
            private TableLayout layout;

            internal ItemCollection(TableLayout layout) => this.layout = layout;

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
        /// Represents an item (layouted view) in a <see cref="TableLayout" />.
        /// </summary>
        public sealed class Item {
            private readonly TableLayout layout;

            private int column;
            private int row;
            private int columnSpan = 1;
            private int rowSpan = 1;

            internal Item(TableLayout layout, View view) {
                this.layout = layout;
                View = view;
            }

            /// <summary>
            /// Retrieves the view associated with this item.
            /// </summary>
            /// <returns>The view associated with this item.</returns>
            public View View { get; }

            /// <summary>
            /// Sets or retrieves the first column index where this item is located.
            /// </summary>
            /// <exception name="ArgumentOutOfRangeException">The value is less than zero.
            /// </exception>
            /// <returns>The first column index (0-based).</returns>
            public int Column {
                get => column;
                set {
                    if (value < 0) {
                        throw new ArgumentOutOfRangeException();
                    }
                    if (value == column) {
                        return;
                    }
                    column = value;
                    layout.InvalidateLayout();
                }
            }

            /// <summary>
            /// Sets or retrieves the first row index where this item is located.
            /// </summary>
            /// <exception name="ArgumentOutOfRangeException">The value is less than zero.
            /// </exception>
            /// <returns>The first row index (0-based).</returns>
            public int Row {
                get => row;
                set {
                    if (value < 0) {
                        throw new ArgumentOutOfRangeException();
                    }
                    if (value == row) {
                        return;
                    }
                    row = value;
                    layout.InvalidateLayout();
                }
            }

            /// <summary>
            /// Sets or retrieves the number of columns occpied by this item.
            /// </summary>
            /// <exception name="ArgumentOutOfRangeException">The value is less than one.
            /// </exception>
            /// <returns>The number of columns.</returns>
            public int ColumnSpan {
                get => columnSpan;
                set {
                    if (value < 1) {
                        throw new ArgumentOutOfRangeException();
                    }
                    if (value == columnSpan) {
                        return;
                    }
                    columnSpan = value;
                    layout.InvalidateLayout();
                }
            }

            /// <summary>
            /// Sets or retrieves the number of rows occpied by this item.
            /// </summary>
            /// <exception name="ArgumentOutOfRangeException">The value is less than one.
            /// </exception>
            /// <returns>The number of rows.</returns>
            public int RowSpan {
                get => rowSpan;
                set {
                    if (value < 1) {
                        throw new ArgumentOutOfRangeException();
                    }
                    if (value == rowSpan) {
                        return;
                    }
                    rowSpan = value;
                    layout.InvalidateLayout();
                }
            }

            /// <summary>
            /// Sets or retrives flags indicating how its associated view is positioned within
            /// this item.
            /// </summary>
            /// <returns>The alignment flags.</returns>
            public Alignment Alignment { get; set; }
        }

        /// <summary>
        /// Retrieves the collection of items in this layout object.
        /// </summary>
        /// <returns>The collection of items in this layout object.</returns>
        public ItemCollection Items { get; }

        /// <summary>
        /// Retrieves the collection of columns in this layout object.
        /// </summary>
        /// <returns>The collection of columns in this layout object.</returns>
        public ColumnCollection Columns { get; }

        /// <summary>
        /// Retrieves the collection of rows in this layout object.
        /// </summary>
        /// <returns>The collection of rows in this layout object.</returns>
        public RowCollection Rows { get; }

        private Padding padding;

        /// <summary>
        /// Sets or retrieves the padding values of this layout object.
        /// </summary>
        /// <returns>The padding values.</returns>
        public Padding Padding {
            get => padding;
            set {
                padding = value;
                InvalidateLayout();
            }
        }

        /// <summary>
        /// Initializes a new instance of <see cref="TableLayout" />.
        /// </summary>
        public TableLayout() {
            Items = new ItemCollection(this);
            Columns = new ColumnCollection(this);
            Rows = new RowCollection(this);
        }

        /// <summary>
        /// Implements <see cref="Layout.Subviews" />.
        /// </summary>
        public override IEnumerable<View> Subviews { get => Items.Subviews; }

        /// <summary>
        /// Implements <see cref="Layout.Measure" />.
        /// </summary>
        public override Measurement Measure() {
            throw new NotImplementedException(); // TODO: Implement `TableLayout`
        }

        /// <summary>
        /// Implements <see cref="Layout.Arrange" />.
        /// </summary>
        public override void Arrange(ArrangeContext context) {
            throw new NotImplementedException(); // TODO: Implement `TableLayout`
        }
    }
}
