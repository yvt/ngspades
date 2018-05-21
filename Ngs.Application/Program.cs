//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using Ngs.UI;
using Ngs.Engine.Presentation;
using Ngs.Utils;

namespace Ngs.Shell {
    sealed class Application : Ngs.Application {
        public static void Main(string[] args) {
            var thisApp = new Application();
            thisApp.Run();
        }

        private Application() {
            this.ApplicationInfo.Name = "Nightingales Test Application";
        }

        sealed class MainView : View {
            protected override void RenderContents(RenderContext context) {
                context.EmitLayer(new SolidColorLayerInfo()
                {
                    Bounds = new Box2(20, 20, 20, 20),
                    FillColor = Rgba.White,
                });
            }
        }

        private void Run() {
            Console.WriteLine("Displaying some window");

            this.UIQueue.Invoke(() => {
                var window = new Window()
                {
                    ContentsView = new MainView(),
                };

                window.Visible = true;
            });

            this.Start();
        }
    }
}