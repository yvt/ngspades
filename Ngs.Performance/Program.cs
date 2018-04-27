using System;
using BenchmarkDotNet.Running;

namespace Ngs.Performance {
    class Program {
        static void Main(string[] args) {
            var switcher = new BenchmarkSwitcher(new[] {
                typeof(PresentationPropertyBenchmark),
            });
            switcher.Run(args);
        }
    }
}
