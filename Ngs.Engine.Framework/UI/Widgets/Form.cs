//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.Engine.UI.Widgets {
    public abstract class Form : View {
        // TODO

        protected abstract void Design();

        internal override void BeforeLayout() {
        }
    }
}
