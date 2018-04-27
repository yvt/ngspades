//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using BenchmarkDotNet.Attributes;
using Ngs.Utils;
using Ngs.Engine;
using Ngs.Engine.Presentation;

namespace Ngs.Performance {
    public class PresentationPropertyBenchmark {
        IWorkspace workspace;
        ILayer materializedLayer;

        public PresentationPropertyBenchmark() {
            workspace = EngineInstance.NativeEngine.CreateWorkspace();
            materializedLayer = workspace.Context.CreateLayer();

            // For node materialization
            workspace.Context.CreateLayer().Child = materializedLayer;
        }

        [Benchmark(OperationsPerInvoke = 10000)]
        public void SetSolidColorOnMaterializedLayer() {
            for (long i = 0; i < 10000; ++i) {
                materializedLayer.SolidColor = Rgba.TransparentBlack;
            }
        }

        [Benchmark(OperationsPerInvoke = 10000)]
        public void SetSolidColorOnMaterializedLayerWithLock() {
            workspace.Context.Lock();
            for (long i = 0; i < 10000; ++i) {
                materializedLayer.SolidColor = Rgba.TransparentBlack;
            }
            workspace.Context.Unlock();
        }
    }
}
