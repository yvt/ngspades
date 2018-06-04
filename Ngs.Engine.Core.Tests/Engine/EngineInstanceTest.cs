//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using Xunit;

namespace Ngs.Engine.Tests {
    public class EngineInstanceTest {
        [Fact]
        public void CanLoad() {
            EngineInstance.EnsureLoaded();
        }
    }
}
