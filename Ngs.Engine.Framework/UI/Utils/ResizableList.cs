//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Collections;
using System.Collections.Generic;
using System.ComponentModel;

namespace Ngs.Engine.UI.Utils {
    /// <summary>
    /// Nightingales infrastructure. Not intended to be directly used by an application.
    /// Represents a collection that uses a framework-supplied trait type to instantiate its elements.
    /// </summary>
    /// <typeparam name="T">The element type.</typeparam>
    [EditorBrowsable(EditorBrowsableState.Never)]
    public abstract class ResizableList<T> : IReadOnlyList<T> {
        readonly List<T> items = new List<T>();
        readonly int maxCount;

        internal ResizableList(int maxCount) {
            this.maxCount = maxCount;
        }

        /// <summary>
        /// Called when the collection was modified.
        /// </summary>
        protected abstract void OnUpdate();

        /// <summary>
        /// Requests to create a new instance of the item type.
        /// </summary>
        protected abstract T CreateItem();

        /// <summary>
        /// Retrieves the item at the specified index in this collection.
        /// </summary>
        /// <param name="index">The index of the item to retrieve.</param>
        /// <returns>The item located at the specified index.</returns>
        public T this[int index] {
            get => items[index];
        }

        /// <summary>
        /// Sets and retrieves the number of items in this collection.
        /// </summary>
        /// <exception name="ArgumentOutOfRangeException"><c>value</c> exceeds an
        /// implementation-defined limit, or is less than zero.</exception>
        /// <returns>The number of items in this collection.</returns>
        public int Count {
            get => items.Count;
            set {
                if (value < 0 || value > maxCount) {
                    throw new ArgumentOutOfRangeException();
                }
                bool updated = value != items.Count;
                while (value > items.Count) {
                    items.Add(CreateItem());
                }
                items.RemoveRange(value, items.Count - value);

                if (updated) {
                    OnUpdate();
                }
            }
        }

        /// <summary>
        /// Returns an enumerator object that is used to iterate over this collection.
        /// </summary>
        /// <returns>An enumerator object.</returns>
        public IEnumerator<T> GetEnumerator() => items.GetEnumerator();
        IEnumerator IEnumerable.GetEnumerator() => items.GetEnumerator();
    }
}
