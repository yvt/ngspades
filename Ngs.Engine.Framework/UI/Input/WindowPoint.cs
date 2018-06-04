//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using Ngs.Engine;

namespace Ngs.UI.Input {
    sealed class WindowPoint : Point {
        Window window;
        Vector2 clientPosition;

        public WindowPoint(Window window, Vector2 clientPosition) {
            this.window = window ?? throw new ArgumentNullException(nameof(window));
            this.clientPosition = clientPosition;
        }

        public override Vector2? GetPositionInWindowClient(Window window) =>
            window == this.window ? this.clientPosition : (Vector2?)null;
    }
}