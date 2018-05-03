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
        Workspace workspace;
        Ngs.Engine.Native.INgsPFLayer materializedLayer;

        public PresentationPropertyBenchmark() {
            workspace = new Workspace(new ApplicationInfo()
            {
                Name = "Ngs.Performance"
            });
            materializedLayer = workspace.Context.CreateLayer();

            // For node materialization
            workspace.Context.CreateLayer().Child = materializedLayer;
        }

        [Benchmark(OperationsPerInvoke = 10000)]
        public void SetOpacityOnMaterializedLayer() {
            for (long i = 0; i < 10000; ++i) {
                materializedLayer.Opacity = 0.0f;
            }
        }

        [Benchmark(OperationsPerInvoke = 10000)]
        public void SetOpacityOnMaterializedLayerWithLock() {
            workspace.Context.Lock();
            for (long i = 0; i < 10000; ++i) {
                materializedLayer.Opacity = 0.0f;
            }
            workspace.Context.Unlock();
        }

        [Benchmark(OperationsPerInvoke = 10000)]
        public void SetSolidColorOnMaterializedLayer() {
            for (long i = 0; i < 10000; ++i) {
                materializedLayer.SetContentsSolidColor(Rgba.TransparentBlack);
            }
        }

        [Benchmark(OperationsPerInvoke = 10000)]
        public void SetSolidColorOnMaterializedLayerWithLock() {
            workspace.Context.Lock();
            for (long i = 0; i < 10000; ++i) {
                materializedLayer.SetContentsSolidColor(Rgba.TransparentBlack);
            }
            workspace.Context.Unlock();
        }
    }
}
