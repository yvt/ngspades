//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using Ngs.Engine.Presentation;
using Ngs.Utils;

namespace Ngs.Shell {
    class MainClass {
        public static void Main(string[] args) {
            Console.WriteLine("Displaying some window");
            var ws = new Workspace(new ApplicationInfo()
            {
                Name = "Nightingales Test Application",
                VersionMajor = 1,
                VersionMinor = 0,
                VersionRevision = 0,
            });

            // Create and display a window
            ws.Context.Lock();

            var window = ws.Context.CreateWindow();

            var layer = ws.Context.CreateLayer();
            layer.Bounds = new Box2(20, 20, 20, 20);
            layer.SolidColor = Rgba.White;
            window.Child = layer;

            ws.Windows = window;

            ws.Context.Unlock();
            ws.Context.CommitFrame();

            new System.Threading.Thread(() => {
                System.Threading.Thread.Sleep(1000);
                ws.Exit();
                ws.Context.CommitFrame();
            }).Start();
            ws.Start();
        }
    }
}