//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Collections;
using System.Collections.Generic;
using System.ComponentModel;

namespace Ngs.UI.Utils {
    /// <summary>
    /// Nightingales infrastructure. Not intended to be directly used by an application.
    /// </summary>
    /// <typeparam name="T">The layout item type.</typeparam>
    [EditorBrowsable(EditorBrowsableState.Never)]
    public abstract class LayoutItemCollection<T> : IEnumerable<T> {
        readonly Dictionary<View, T> items = new Dictionary<View, T>();

        internal LayoutItemCollection() { }

        /// <summary>
        /// Called when the collection was modified.
        /// </summary>
        protected abstract void OnUpdate();

        /// <summary>
        /// Called before a view is added to this collection.
        /// </summary>
        /// <param name="view">The view that is being added.</param>
        protected abstract void OnViewBeingAdded(View view);

        /// <summary>
        /// Called before a view is removed from this collection.
        /// </summary>
        /// <param name="view">The view that is being removed.</param>
        protected abstract void OnViewBeingRemoved(View view);

        /// <summary>
        /// Requests to create a new instance of the item type.
        /// </summary>
        protected abstract T CreateItem(View view);

        /// <summary>
        /// Returns an enumerator object that is used to iterate over this collection.
        /// </summary>
        /// <returns>An enumerator object.</returns>
        public IEnumerator<T> GetEnumerator() => items.Values.GetEnumerator();
        IEnumerator IEnumerable.GetEnumerator() => items.Values.GetEnumerator();

        /// <summary>
        /// Retrieves the number of items in this collection.
        /// </summary>
        /// <returns>The number of items in this collection</returns>
        public int Count { get => items.Count; }

        /// <summary>
        /// Retrieves the item associated with the specified view.
        /// </summary>
        /// <exception name="KeyNotFoundException">The view does not exist in this collection.
        /// </exception>
        /// <returns>The item associated with the specified view</returns>
        public T this[View view] {
            get => items[view];
        }

        /// <summary>
        /// Adds the specified view to this collection.
        /// </summary>
        /// <param name="view">The view to add.</param>
        /// <exception name="ArgumentNullException"><paramref name="view" /> is <c>null</c>.
        /// </exception>
        /// <exception name="ArgumentException">The view already exists in this collection.
        /// </exception>
        /// <returns>The newly created item that is associated with the view.</returns>
        public T Add(View view) {
            if (view == null) {
                throw new ArgumentNullException(nameof(view));
            }

            OnViewBeingAdded(view);

            var item = CreateItem(view);
            items.Add(view, item);

            OnUpdate();

            return item;
        }

        /// <summary>
        /// Removes the specified view from this collection.
        /// </summary>
        /// <param name="view">The view to remove.</param>
        /// <exception name="ArgumentNullException"><paramref name="view" /> is <c>null</c>.
        /// </exception>
        /// <returns><c>true</c> if the view is successfully found and remove; otherwise,
        /// <c>false</c>.</returns>
        public bool Remove(View view) {
            if (!items.ContainsKey(view)) {
                return false;
            }
            OnViewBeingRemoved(view);
            items.Remove(view);
            OnUpdate();
            return true;
        }

        /// <summary>
        /// Retrieves the list of views in this collection.
        /// </summary>
        /// <returns>The list of views in this collection.</returns>
        public Dictionary<View, T>.KeyCollection Subviews { get => items.Keys; }
    }
}
