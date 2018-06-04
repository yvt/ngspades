//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using Ngs.Engine;

namespace Ngs.Engine.UI.Input {
    /// <summary>
    /// Represents a point indicated by a pointing device, such as a mouse or touch screen.
    /// </summary>
    public abstract class Point {
        /// <summary>
        /// Retrieves the position of this point in relative to a sepcified window's client
        /// coordinate space.
        /// </summary>
        /// <param name="window">The window.</param>
        /// <returns>The position relative to the client coordinate space of
        /// <paramref name="window" />. <c>null</c> if the conversion was unsccessful, for example,
        /// because the relation between the original coordinate space and the window's coordinate
        /// space is unknown.</returns>
        public abstract Vector2? GetPositionInWindowClient(Window window);

        /// <summary>
        /// Retrieves the position of this point in relative to a sepcified view's local coordinate
        /// space.
        /// </summary>
        /// <param name="view">The view.</param>
        /// <returns>The position relative to <paramref name="view" />. <c>null</c> if the
        /// conversion was unsccessful, for example, because the relation between the original
        /// coordinate space and the view's coordinate space is unknown.</returns>
        public Vector2? GetPositionInView(View view) {
            Window window = view.Window;
            if (window == null) {
                return null;
            }

            Vector2? clientP = this.GetPositionInWindowClient(window);
            if (clientP == null) {
                return null;
            }

            Matrix3 clientToLocalM = view.GetWindowClientToLocalPlaneTransform().Value;
            return clientToLocalM.TransformPoint(clientP.Value);
        }
    }
}
