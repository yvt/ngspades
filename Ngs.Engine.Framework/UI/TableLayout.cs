//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Collections;
using System.Collections.Generic;
using System.Diagnostics;
using System.Numerics;
using System.ComponentModel;
using System.Linq;
using Ngs.Utils;
using Ngs.UI.Utils;

namespace Ngs.UI {
    /// <summary>
    /// Layouts subviews using an imaginary table composed of rows and columns.
    /// </summary>
    /// <remarks>
    /// The implementation sets a hard limit on the size of the table as each row and column
    /// generates a constant runtime overhead. The application should try to minimize the size of
    /// the table for best performance.
    /// </remarks>
    public sealed class TableLayout : Layout {
        /// <summary>
        /// The maximum value that can be used as a part of an item position specification.
        /// </summary>
        const int ADDRESS_LIMIT = 4096;

        /// <summary>
        /// The maximum possible size of the layout table.
        /// </summary>
        const int MAX_TABLE_SIZE = ADDRESS_LIMIT * 2 - 1;

        /// <summary>
        /// Represents the end of a list.
        /// </summary>
        const int EOL = -1;

        /// <summary>
        /// Represents a collection of <see cref="Column" /> objects in a <see cref="TableLayout" />.
        /// </summary>
        public sealed class ColumnCollection : ResizableList<Column> {
            private readonly TableLayout layout;
            internal ColumnCollection(TableLayout layout) : base(MAX_TABLE_SIZE) =>
                this.layout = layout;

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
            internal RowCollection(TableLayout layout) : base(MAX_TABLE_SIZE) =>
                this.layout = layout;

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

            /// <summary>
            /// Implements <see cref="LayoutItemCollection&lt;T&gt;.OnViewBeingAdded" />.
            /// </summary>
            protected override void OnViewBeingAdded(View view) {
                layout.AttachSubview(view);
                layout.itemListDirty = true;
            }

            /// <summary>
            /// Implements <see cref="LayoutItemCollection&lt;T&gt;.OnViewBeingRemoved" />.
            /// </summary>
            protected override void OnViewBeingRemoved(View view) {
                layout.DetachSubview(view);
                layout.itemListDirty = true;
            }
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
            /// <exception name="ArgumentOutOfRangeException">The value is not in range
            /// <c>[0, 4095]</c>.</exception>
            /// <returns>The first column index (0-based).</returns>
            public int Column {
                get => column;
                set {
                    if (value < 0 || value >= ADDRESS_LIMIT) {
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
            /// <exception name="ArgumentOutOfRangeException">The value is not in range
            /// <c>[0, 4095]</c>.</exception>
            /// <returns>The first row index (0-based).</returns>
            public int Row {
                get => row;
                set {
                    if (value < 0 || value >= ADDRESS_LIMIT) {
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
            /// <exception name="ArgumentOutOfRangeException">The value is not in range
            /// <c>[1, 4096]</c>.</exception>
            /// <returns>The number of columns.</returns>
            public int ColumnSpan {
                get => columnSpan;
                set {
                    if (value < 1 || value > ADDRESS_LIMIT) {
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
            /// <exception name="ArgumentOutOfRangeException">The value is not in range
            /// <c>[1, 4096]</c>.</exception>
            /// <returns>The number of rows.</returns>
            public int RowSpan {
                get => rowSpan;
                set {
                    if (value < 1 || value > ADDRESS_LIMIT) {
                        throw new ArgumentOutOfRangeException();
                    }
                    if (value == rowSpan) {
                        return;
                    }
                    rowSpan = value;
                    layout.InvalidateLayout();
                }
            }

            internal (int, int) ColumnRowRange(int axis) {
                switch (axis) {
                    case 0: return (Column, ColumnSpan);
                    case 1: return (Row, RowSpan);
                    default: throw new ArgumentOutOfRangeException();
                }
            }

            /// <summary>
            /// Sets or retrives flags indicating how its associated view is positioned within
            /// this item.
            /// </summary>
            /// <returns>The alignment flags.</returns>
            public Alignment Alignment { get; set; }

            // ==== The following fields are internally used by `TableLayout` =====

            // Points the next item in a singly-linked list starting with `Line.originListFirst`.
            // `-1` if this item is the last;
            internal int rowOriginListNext;
            // Points the next item in a singly-linked list starting with `Line.originListFirst`.
            // `-1` if this item is the last;
            internal int columnOriginListNext;

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
        /// The random-accessible list, constructed in <c>Measure</c>.
        /// </summary>
        readonly List<Item> itemList = new List<Item>();

        bool itemListDirty;

        // Represents a row or column.
        struct Line {
            public float minimumPosition;
            public float maximumPosition;

            /// <summary>
            /// Copied from `Row` or `Column`.
            /// </summary>
            public float? preferredSize;

            public float size;
            public float position;

            // Points the first item in `itemList` that have its `Row` or `Column` pointing this `Line`.
            public int originListFirst;

            public static readonly Line Default = new Line()
            {
                maximumPosition = float.PositiveInfinity,
                originListFirst = EOL,
            };
        }

        // The actual row/column counts
        int numRows;
        int numColumns;

        // `rowLines.Length >= numRows + 1`
        Line[] rowLines;
        // `columnLines.Length >= numColumns + 1`
        Line[] columnLines;

        /// <summary>
        /// Calculates the optimal values for <c>rowLines[n].size</c> and
        /// <c>columnLines[n].size</c> by applying a linear programming algorithm.
        /// </summary>
        void Optimize(int axis, Span<Line> lines, float? totalWidth) {
            // The goal of this algorithm is to minimize the difference between the preferred value
            // and the actual width/height of each column/row without violating the hard constraints.
            // This problem can be formulated as a linear programming problem as described below:
            //
            //  - Objective function: Minimize the sum of the error (`Eₙ = abs(wₙ - Wₙ) / Wₙ`)
            //    between the preferred values (`Wₙ`) and the actual row/column sizes (`wₙ`).
            //
            //    The preference values set on `Row`s and `Column`s are prioritized.
            //
            //  - Constraint 1: For every layouted item, the total size of the covered rows and
            //    columns must not be less than the item's minimum size.
            //
            //  - Constraint 2: For every layouted item, the total size of the covered rows and
            //    columns must not exceed the item's maximum size if the item's alignment mode of
            //    the corresponding axis is set to justify.
            //
            //  - Constraint 3: No row and columns shall have negative sizes.
            //
            //  - Constraint 4: If this algorithm is executed as a part of the arrange layout phase,
            //    then the total size of all rows or columns must be equal to the requested size.
            //
            // This can be converted into a standard form as following:
            //
            //  - Objective function: For each preference value (`Wₙ`), introduce new variables
            //    `W⁺ₙ` and `W⁻ₙ` and an equality constraint `wₙ = Wₙ + W⁺ₙ - W⁻ₙ` (Constraint 5).
            //    Replace the corresponding term in the original objective function with
            //    `(W⁺ₙ + W⁻ₙ) / Wₙ`.
            //
            //  - Add a slack variable to each of constraint defined by Constraint 1 and 2 and
            //    convert it to an equality constraint.
            //
            //  - To skip the canonicalization step and simplify the implementation, add more slack
            //    variables (`errorMinᵢ` and `errorTotal`) to Constraint 1 and 4.
            //    These slack variables contribute to the objective function, and their weight is
            //    infinitely large (`RANK_HARD_LIMIT`).
            //
            // This transformation produces the following revised problem description:
            //
            //  - `Ω` is an infinitely large value.
            //  - Objective function: `sum [(W⁺ₙ + W⁻ₙ) * (1 / Wₙ) * Ω^(rₙ) for each preferred value Wₙ]`
            //    `+ Ω³ (errorMinᵢ + errorTotal)`
            //  - Constraint 1: For each item `i`, `sum[w[k..l]] - slackMinᵢ + errorMinᵢ = minWidthᵢ`
            //  - Constraint 2: For each item `i`, `sum[w[k..l]] + slackMaxᵢ = maxWidthᵢ`
            //    if `maxWidthᵢ` is defined.
            //  - Constraint 4: `sum[w] + errorTotal = totalWidth` if `totalWidth` is defined.
            //  - (All variables are implicitly greater than or equal to 0)
            //  - Constraint 5: For each preferred value, `sum[w[k..l]] - W⁺ₙ + W⁻ₙ = Wₙ`
            //
            // This can be solved directly by the simplex algorithm because this description is
            // already in canonical form. Here's the list of the variables in this program:
            //
            //  - `|{wᵢ}| == cellCount`
            //  - `|{slackMinᵢ}| == itemCount`
            //  - `|{slackMaxᵢ}| == maxConstrainedItemCount` (initial basic variable)
            //  - `|{errorMinᵢ}| == itemCount` (initial basic variable)
            //  - `|{W⁺ₙ}| == cellCount + itemCount`
            //  - `|{W⁻ₙ}| == cellCount + itemCount` (initial basic variable)
            //  - `|{errorTotal}| == 1` (optional, initial basic variable)
            //
            // TODO: For now this implementation is based on the textbook simplex method. The memory
            //       consumption for the maintaining tableau grows in the quadratic order, so this
            //       approach is not feasible even when the number of items is not so large.
            //       There's a room of improvement here as the sparsity of the constraint matrix
            //       can be exploited.
            //
            // TODO: As it turned out, there are multiple critical flaws in using the linear
            //       programming for solving the table layout optimization problem: the first issue
            //       is described above; the second one is that it always chooses the extreme values,
            //       for example, it's impossible to size two columns evenly - the result is one of
            //       them having the minimum possible size while the other taking up entire the
            //       remaining area.
            //       Using the sum of squared errors as the alternative objective function and
            //       applying a quadratic programming technique will solve this issue.
            var items = this.Items;
            int cellCount = lines.Length - 1;
            int itemCount = items.Count;

            Alignment justifyMask = axis == 1 ?
                Alignment.VerticalJustify : Alignment.HorizontalJustify;

            int maxConstrainedItemCount = items.Count((item) =>
                float.IsFinite(item.View.Measurement.MaximumSize.GetElementAt(axis)) &&
                (item.Alignment & justifyMask) == justifyMask);

            // Variable indices
            int indexWidth = 1;
            int indexSlackMin = indexWidth + cellCount;
            int indexSlackMax = indexSlackMin + itemCount;
            int indexErrorMin = indexSlackMax + maxConstrainedItemCount;
            int indexResiduePos = indexErrorMin + itemCount;
            int indexResidueNeg = indexResiduePos + cellCount + itemCount;
            int indexErrorTotal = indexResidueNeg + cellCount + itemCount;
            int indexGoal = indexErrorTotal + (totalWidth.HasValue ? 1 : 0);

#if false
            Console.WriteLine($"indexWidth = {indexWidth}");
            Console.WriteLine($"indexSlackMin = {indexSlackMin}");
            Console.WriteLine($"indexSlackMax = {indexSlackMax}");
            Console.WriteLine($"indexErrorMin = {indexErrorMin}");
            Console.WriteLine($"indexResiduePos = {indexResiduePos}");
            Console.WriteLine($"indexResidueNeg = {indexResidueNeg}");
            Console.WriteLine($"indexErrorTotal = {indexErrorTotal}");
            Console.WriteLine($"indexGoal = {indexGoal}");
#endif

            System.Runtime.CompilerServices.RuntimeHelpers.EnsureSufficientExecutionStack();

            // Construct tableau
            int numVars = indexGoal;
            int numConstraints = cellCount + itemCount * 2 + maxConstrainedItemCount + (totalWidth.HasValue ? 1 : 0);

            int tableauSize = checked((numConstraints + 1) * (numVars + 1));
            Span<double> tableau = tableauSize > 4096 ?
                new double[tableauSize] : stackalloc double[tableauSize];
            var objectiveRow = tableau.Slice(0, numVars + 1);
            int rowConstraint1;
            int rowConstraint2;
            int rowConstraint4;
            int rowConstraint5;

            {
                int rowIndex = 0;
                // Objective function
                {
                    var row = tableau.Slice((rowIndex++) * (numVars + 1), numVars + 1);

                    const double OMEGA = 16384;
                    const double RANK_HARD_LIMIT = OMEGA * OMEGA * OMEGA;
                    const double RANK_LAYOUT = OMEGA * OMEGA;
                    const double RANK_VIEW = OMEGA;

                    int index = 0;
                    row[index++] = 1;
                    index += cellCount; // wᵢ
                    index += itemCount + maxConstrainedItemCount; // slackMinᵢ, slackMaxᵢ
                    // errorMinᵢ
                    for (int i = 0; i < itemCount; ++i) {
                        row[index++] = RANK_HARD_LIMIT;
                    }
                    // W⁺ₙ, W⁻ₙ
                    for (int pm = 0; pm < 2; ++pm) {
                        for (int i = 0; i < cellCount; ++i) {
                            if (lines[i].preferredSize is float preferredSizeVal) {
                                if (preferredSizeVal == 0) {
                                    row[index++] = RANK_HARD_LIMIT;
                                } else {
                                    row[index++] = RANK_LAYOUT / preferredSizeVal;
                                }
                            } else {
                                row[index++] = 1;
                            }
                        }
                        foreach (var item in items) {
                            float preferredSizeVal = item.View.Measurement
                                .PreferredSize.GetElementAt(axis);
                            if (preferredSizeVal == 0) {
                                // FIXME: I started to feel this is a bad idea
                                row[index++] = RANK_HARD_LIMIT;
                            } else {
                                row[index++] = RANK_VIEW / preferredSizeVal;
                            }
                        }
                    }
                    // errorTotal
                    if (totalWidth is float totalWidthVal2) {
                        row[index++] = RANK_HARD_LIMIT;
                    }
                }

                // Constaint 1
                rowConstraint1 = rowIndex;
                {
                    int itemIndex = 0;
                    foreach (var item in items) {
                        var row = tableau.Slice((rowIndex++) * (numVars + 1), numVars + 1);

                        (int start, int count) = item.ColumnRowRange(axis);

                        row.Slice(start + indexWidth, count).Fill(1);
                        row[indexSlackMin + itemIndex] = -1;
                        row[indexErrorMin + itemIndex] = 1;
                        row[indexGoal] = item.View.Measurement.MinimumSize.GetElementAt(axis);

                        itemIndex += 1;
                    }
                }

                // Constaint 2
                rowConstraint2 = rowIndex;
                {
                    int itemIndex = 0;
                    foreach (var item in items) {
                        if (
                            float.IsFinite(item.View.Measurement.MaximumSize.GetElementAt(axis)) &&
                            (item.Alignment & justifyMask) == justifyMask
                        ) {
                            var row = tableau.Slice((rowIndex++) * (numVars + 1), numVars + 1);

                            (int start, int count) = item.ColumnRowRange(axis);

                            row.Slice(start + indexWidth, count).Fill(1);
                            row[indexSlackMax + itemIndex] = 1;
                            row[indexGoal] = item.View.Measurement.MaximumSize.GetElementAt(axis);
                            itemIndex += 1;
                        }
                    }
                }

                // Constraint 3 is automatically enforced by the definition of linear programming.

                // Constraint 4
                rowConstraint4 = rowIndex;
                if (totalWidth is float totalWidthVal) {
                    var row = tableau.Slice((rowIndex++) * (numVars + 1), numVars + 1);

                    row.Slice(indexWidth, cellCount).Fill(1);
                    row[indexErrorTotal] = 1;
                    row[indexGoal] = totalWidthVal;
                }

                // Constraint 5 (cell)
                rowConstraint5 = rowIndex;
                for (int i = 0; i < cellCount; ++i) {
                    var row = tableau.Slice((rowIndex++) * (numVars + 1), numVars + 1);
                    row[indexWidth + i] = 1;
                    row[indexResiduePos + i] = -1;
                    row[indexResidueNeg + i] = 1;
                    row[indexGoal] = lines[i].preferredSize ?? 0;
                }
                {
                    int itemIndex = 0;
                    foreach (var item in items) {
                        var row = tableau.Slice((rowIndex++) * (numVars + 1), numVars + 1);

                        (int start, int count) = item.ColumnRowRange(axis);

                        row.Slice(start + indexWidth, count).Fill(1);
                        row[indexWidth + cellCount + itemIndex] = 1;
                        row[indexResiduePos + cellCount + itemIndex] = -1;
                        row[indexResidueNeg + cellCount + itemIndex] = 1;
                        row[indexGoal] = item.View.Measurement.PreferredSize.GetElementAt(axis);

                        itemIndex += 1;
                    }
                }

                Debug.Assert(rowIndex == numConstraints + 1);
            }

#if false
            {
                for (int k = 0; k < numConstraints + 1; ++k) {
                    var row = tableau.Slice(k * (numVars + 1), numVars + 1);
                    foreach (var e in row) {
                        Console.Write("{0,6}", e);
                    }
                    Console.WriteLine();
                }
            }
#endif

            // Do "pricing out"
            // Must be ordered by the variable index (the first element of the tuple)
            var initialBasicVarRanges = new[] {
                (indexSlackMax, maxConstrainedItemCount, rowConstraint2, false),
                (indexErrorMin, itemCount, rowConstraint1, true),
                (indexResidueNeg, cellCount + itemCount, rowConstraint5, true),
                (indexErrorTotal, totalWidth.HasValue ? 1 : 0, rowConstraint4, true),
            };
            foreach (
                (int varStart, int count, int rowStart, bool factorNonZero) in initialBasicVarRanges
            ) {
                if (!factorNonZero) {
                    // This basic variables do not contribute to the objective function
                    // (`objectiveRow[varIndex..varIndex+count]` is zero)
                    continue;
                }
                for (
                    int i = 0, varIndex = varStart, rowIndex = rowStart;
                    i < count;
                    ++i, ++varIndex, ++rowIndex
                ) {
                    var row = tableau.Slice(rowIndex * (numVars + 1), numVars + 1);
                    var factor = objectiveRow[varIndex];
                    Debug.Assert(row[varIndex] == 1);
                    for (int k = 0; k < numVars + 1; ++k) {
                        objectiveRow[k] -= row[k] * factor;
                    }
                    objectiveRow[varIndex] = 0;
                }
            }

            // The current set of basic variables
            Span<int> basicVars = numConstraints > 4096 ?
                new int[numConstraints] : stackalloc int[numConstraints];
            Span<int> basicVarRows = numConstraints > 4096 ?
                new int[numConstraints] : stackalloc int[numConstraints]; // row index (never 0)
            {
                int index = 0;
                foreach ((int varStart, int count, int rowStart, _) in initialBasicVarRanges) {
                    for (
                        int i = 0, varIndex = varStart, rowIndex = rowStart;
                        i < count;
                        ++i, ++varIndex, ++rowIndex
                    ) {
                        basicVars[index] = varIndex;
                        basicVarRows[index] = rowIndex;
                        ++index;
                    }
                }
                Debug.Assert(index == basicVars.Length);
            }

            // The current set of non-basic variables
            Span<int> nonBasicVars = stackalloc int[numVars - numConstraints - 1];
            {
                int k = 0, index = 0;
                for (int varIndex = 1; varIndex < numVars;) {
                    if (k < initialBasicVarRanges.Length && varIndex == initialBasicVarRanges[k].Item1) {
                        // `i` is a basic variable - skip it
                        varIndex += initialBasicVarRanges[k].Item2;
                        k += 1;
                        continue;
                    }
                    nonBasicVars[index++] = varIndex;
                    varIndex += 1;
                }
                Debug.Assert(index == nonBasicVars.Length);
            }

            // The main loop of the algorithm
#if false
            Console.WriteLine($"Starting pivotting with totalWidth={totalWidth}");
#endif
            while (true) {
#if false
                for (int k = 0; k < numConstraints + 1; ++k) {
                    var row = tableau.Slice(k * (numVars + 1), numVars + 1);
                    foreach (var e in row) {
                        Console.Write("{0,6}", e);
                    }
                    Console.WriteLine();
                }
#endif

                // Choose the entering variable based on Blend's rule.
                // Bland, Robert G. (May 1977). "New finite pivoting rules for the simplex method".
                // Mathematics of Operations Research. 2 (2): 103–107.
                int enteringNBVIndex = 0;
                for (; enteringNBVIndex < nonBasicVars.Length; ++enteringNBVIndex) {
                    if (objectiveRow[nonBasicVars[enteringNBVIndex]] < 0) {
                        break;
                    }
                }
                if (enteringNBVIndex == nonBasicVars.Length) {
                    // The solution is in fact optimal
                    break;
                }
                int enteringVarIndex = nonBasicVars[enteringNBVIndex];

                // Choose the leaving variable using the minimum ratio test.
                int leavingBVIndex = -1;
                double minimumRatio = double.PositiveInfinity;
                for (int i = 0; i < basicVars.Length; ++i) {
                    var row = tableau.Slice(basicVarRows[i] * (numVars + 1), numVars + 1);
                    var a = row[enteringVarIndex];
                    if (a <= 0) {
                        continue;
                    }
                    var b = row[indexGoal];
                    var ratio = b / a;
                    if (ratio < minimumRatio) {
                        minimumRatio = ratio;
                        leavingBVIndex = i;
                    }
                }
                Debug.Assert(leavingBVIndex != -1); // The objective function must not be unbounded

                // Perform the pivot operation
                int pivotRowIndex = basicVarRows[leavingBVIndex];
                var pivotRow = tableau.Slice(pivotRowIndex * (numVars + 1), numVars + 1);
                {
                    var t = pivotRow[enteringVarIndex];
                    for (int i = 1; i < enteringVarIndex; ++i) {
                        pivotRow[i] /= t;
                    }
                    pivotRow[enteringVarIndex] = 1;
                    for (int i = enteringVarIndex + 1; i < pivotRow.Length; ++i) {
                        pivotRow[i] /= t;
                    }
                }

                for (int k = 0; k < numConstraints + 1; ++k) {
                    if (k == pivotRowIndex) {
                        continue;
                    }

                    var row = tableau.Slice(k * (numVars + 1), numVars + 1);
                    var factor = row[enteringVarIndex];
                    if (factor == 0) {
                        continue;
                    }
                    for (int i = 1; i < enteringVarIndex; ++i) {
                        row[i] -= pivotRow[i] * factor;
                    }
                    row[enteringVarIndex] = 0;
                    for (int i = enteringVarIndex + 1; i < row.Length; ++i) {
                        row[i] -= pivotRow[i] * factor;
                    }
                }

                // Update the basic/non-basic variable set
                int leavingVarIndex = basicVars[leavingBVIndex];
                basicVars[leavingBVIndex] = enteringVarIndex;
                nonBasicVars[enteringNBVIndex] = leavingVarIndex;
            }

            // Compute the result
            for (int i = 0; i < cellCount; ++i) {
                lines[i].size = 0; // Non-basic variable
            }
            for (int i = 0; i < basicVars.Length; ++i) {
                int cell = basicVars[i] - indexWidth;
                if (cell < cellCount) {
                    var row = tableau.Slice(basicVarRows[i] * (numVars + 1), numVars + 1);
                    var size = row[indexGoal];
                    Debug.Assert(double.IsFinite(size));
                    lines[cell].size = (float)size;
                }
            }

        }

        /// <summary>
        /// Implements <see cref="Layout.Measure" />.
        /// </summary>
        public override Measurement Measure() {
            var itemList = this.itemList;
            if (itemListDirty) {
                itemList.Clear();
                if (Items.Count > itemList.Capacity) {
                    itemList.Capacity = Items.Count;
                }
                itemList.AddRange(Items);
                itemListDirty = false;
            }

            // Update the actual row/column count
            RowCollection rows = this.Rows;
            ColumnCollection columns = this.Columns;
            int numRows = rows.Count, numColumns = columns.Count;
            foreach (var item in itemList) {
                numColumns = Math.Max(numColumns, item.Column + item.ColumnSpan);
                numRows = Math.Max(numRows, item.Row + item.RowSpan);
            }
            this.numRows = numRows;
            this.numColumns = numColumns;

            // Initialize `(row|column)Lines`
            if (this.rowLines == null || numRows + 1 > this.rowLines.Length) {
                this.rowLines = new Line[numRows + 1];
            }
            if (this.columnLines == null || numColumns + 1 > this.columnLines.Length) {
                this.columnLines = new Line[numColumns + 1];
            }
            var rowLines = this.rowLines.AsSpan(0, numRows + 1);
            var columnLines = this.columnLines.AsSpan(0, numColumns + 1);
            rowLines.Fill(Line.Default);
            columnLines.Fill(Line.Default);

            rowLines[0].maximumPosition = 0;
            columnLines[0].maximumPosition = 0;

            // Bucket-sort items
            for (int i = 0; i < itemList.Count; ++i) {
                var item = itemList[i];

                ref var columnLine = ref columnLines[item.Column];
                item.columnOriginListNext = columnLine.originListFirst;
                columnLine.originListFirst = i;

                ref var rowLine = ref rowLines[item.Row];
                item.rowOriginListNext = rowLine.originListFirst;
                rowLine.originListFirst = i;
            }

            // Compute minimum/maximum widths
            for (int i = 0; i < numColumns; ++i) {
                ref var columnLine = ref columnLines[i];
                for (int index = columnLines[i].originListFirst; index != EOL;) {
                    var item = itemList[index];
                    var measurement = item.View.Measurement;

                    Debug.Assert(item.Column == i);

                    ref var columnLineRight = ref columnLines[i + item.ColumnSpan];
                    columnLineRight.minimumPosition = Math.Max(columnLineRight.minimumPosition,
                        columnLine.minimumPosition + measurement.MinimumSize.X);
                    if ((item.Alignment & Alignment.HorizontalMask) == Alignment.HorizontalJustify) {
                        columnLineRight.maximumPosition = Math.Min(columnLineRight.maximumPosition,
                            columnLine.maximumPosition + measurement.MaximumSize.X);
                    }

                    index = item.columnOriginListNext;
                }

                float? preferredSize = i < columns.Count ? columns[i].Width : null;
                if (preferredSize == 0) {
                    ref var columnLineRight = ref columnLines[i + 1];
                    columnLineRight.minimumPosition = columnLine.minimumPosition;
                    columnLineRight.maximumPosition = columnLine.maximumPosition;
                }
                columnLine.preferredSize = preferredSize;
            }

            for (int i = 0; i < numRows; ++i) {
                ref var rowLine = ref rowLines[i];
                for (int index = rowLines[i].originListFirst; index != EOL;) {
                    var item = itemList[index];
                    var measurement = item.View.Measurement;

                    Debug.Assert(item.Row == i);

                    ref var rowLineBottom = ref rowLines[i + item.RowSpan];
                    rowLineBottom.minimumPosition = Math.Max(rowLineBottom.minimumPosition,
                        rowLine.minimumPosition + measurement.MinimumSize.Y);
                    if ((item.Alignment & Alignment.VerticalMask) == Alignment.VerticalJustify) {
                        rowLineBottom.maximumPosition = Math.Min(rowLineBottom.maximumPosition,
                            rowLine.maximumPosition + measurement.MaximumSize.Y);
                    }

                    index = item.rowOriginListNext;
                }

                float? preferredSize = i < rows.Count ? rows[i].Height : null;
                if (preferredSize == 0) {
                    ref var rowLineBottom = ref rowLines[i + 1];
                    rowLineBottom.minimumPosition = rowLine.minimumPosition;
                    rowLineBottom.maximumPosition = rowLine.maximumPosition;
                }
                rowLine.preferredSize = preferredSize;
            }

            Optimize(0, columnLines, null);
            Optimize(1, rowLines, null);

            float TotalSize(Span<Line> lines) {
                float sum = 0;
                foreach (var line in lines) {
                    sum += line.size;
                }
                return sum;
            }

            Vector2 paddingSize = new Vector2(
                Padding.Left + Padding.Right,
                Padding.Top + Padding.Bottom
            );

            return new Measurement()
            {
                MinimumSize = new Vector2(
                    columnLines[numColumns].minimumPosition,
                    rowLines[numRows].minimumPosition
                ) + paddingSize,
                MaximumSize = new Vector2(
                    columnLines[numColumns].maximumPosition,
                    rowLines[numRows].maximumPosition
                ) + paddingSize,
                PreferredSize = new Vector2(
                    TotalSize(columnLines),
                    TotalSize(rowLines)
                ) + paddingSize,
            };
        }

        /// <summary>
        /// Implements <see cref="Layout.Arrange" />.
        /// </summary>
        public override void Arrange(ArrangeContext context) {
            Vector2 paddingSize = new Vector2(
                Padding.Left + Padding.Right,
                Padding.Top + Padding.Bottom
            );

            int numRows = this.numRows, numColumns = this.numColumns;
            var rowLines = this.rowLines.AsSpan(0, numRows + 1);
            var columnLines = this.columnLines.AsSpan(0, numColumns + 1);

            Optimize(0, columnLines, View.Bounds.Width - paddingSize.X);
            Optimize(1, rowLines, View.Bounds.Height - paddingSize.Y);

            columnLines[0].position = Padding.Left;
            rowLines[0].position = Padding.Top;

            for (int i = 1; i < columnLines.Length; ++i) {
                columnLines[i].position = columnLines[i - 1].position + columnLines[i - 1].size;
            }
            for (int i = 1; i < rowLines.Length; ++i) {
                rowLines[i].position = rowLines[i - 1].position + rowLines[i - 1].size;
            }

            foreach (var item in Items) {
                float x1 = columnLines[item.Column].position;
                float x2 = columnLines[item.Column + item.ColumnSpan].position;
                float y1 = rowLines[item.Row].position;
                float y2 = rowLines[item.Row + item.RowSpan].position;
                var topLeft = new Vector2(x1, y1);
                var bottomRight = new Vector2(x2, y2);

                var hAlign = item.Alignment & Alignment.HorizontalMask;
                var vAlign = item.Alignment & Alignment.VerticalMask;
                var measurement = item.View.Measurement;

                Vector2 cellSize = bottomRight - topLeft;
                Vector2 size = cellSize;
                if (hAlign == Alignment.HorizontalJustify) {
                    size.X = Math.Min(size.X, measurement.MaximumSize.X);
                } else {
                    size.X = Math.Min(size.X, measurement.PreferredSize.X);
                }
                if (vAlign == Alignment.VerticalJustify) {
                    size.Y = Math.Min(size.Y, measurement.MaximumSize.Y);
                } else {
                    size.Y = Math.Min(size.Y, measurement.PreferredSize.Y);
                }

                Vector2 offset;
                switch (hAlign) {
                    case Alignment.Left:
                        offset.X = 0;
                        break;
                    case Alignment.HorizontalCenter:
                    case Alignment.HorizontalJustify:
                        offset.X = (cellSize.X - size.X) * 0.5f;
                        break;
                    case Alignment.Right:
                        offset.X = cellSize.X - size.X;
                        break;
                    default:
                        throw new InvalidOperationException();
                }
                switch (vAlign) {
                    case Alignment.Top:
                        offset.Y = 0;
                        break;
                    case Alignment.VerticalCenter:
                    case Alignment.VerticalJustify:
                        offset.Y = (cellSize.Y - size.Y) * 0.5f;
                        break;
                    case Alignment.Bottom:
                        offset.Y = cellSize.Y - size.Y;
                        break;
                    default:
                        throw new InvalidOperationException();
                }

                context.ArrangeSubview(item.View,
                    new Box2(topLeft + offset, topLeft + offset + size));
            }
        }
    }
}
