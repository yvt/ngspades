//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Collections.Generic;
using System.ComponentModel;

namespace Ngs.Engine.UI.Utils {
    internal static class CollectionExtensions {
        /// <summary>
        /// Removes an item from a specified list in O(1) using the "swap and pop" technique.
        /// </summary>
        public static void SwapAndRemoveAt<T>(this List<T> list, int index) {
            int count = list.Count;
            if (index != count - 1) {
                list[index] = list[count - 1];
            }
            list.RemoveAt(count - 1);
        }
    }
}
