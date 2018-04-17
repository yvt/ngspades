//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using Ngs.Engine;

namespace Ngs.Shell {
    class MainClass {
        public static void Main(string[] args) {
            Console.WriteLine("Checking if the engine core was successfully loaded");
            EngineInstance.EnsureLoaded();
        }
    }
}