//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.IO;
using Ngs.UI;
using Ngs.Engine.Presentation;
using Ngs.Engine.Canvas.Text;
using Ngs.Utils;

namespace Ngs.Shell {
    sealed class Application : Ngs.Application {
        public static void Main(string[] args) {
            using (var thisApp = new Application()) {
                thisApp.Run();
            }
        }

        private Application() { }

        protected override ApplicationInfo ApplicationInfo {
            get => new ApplicationInfo()
            {
                Name = "Nightingales Test Application",
            };
        }

        private static FontConfig CreateFontConfig() {
            byte[] ReadAllBytes(Stream s) {
                using (var ms = new MemoryStream()) {
                    s.CopyTo(ms);
                    return ms.ToArray();
                }
            }

            var assembly = typeof(Application).Assembly;
            var fontData = ReadAllBytes(assembly.GetManifestResourceStream("Fonts.DDin"));
            var font = new Font(fontData);

            var config = new FontConfig();
            config.AddFontFace(font.FontFaces[0], "D-DIN", FontStyle.Normal, 300);

            return config;
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
                var layout = new AbsoluteLayout();
                {
                    var label = new Ngs.UI.Widgets.Label()
                    {
                        Text = "Hello!",
                        TextColor = Rgba.White,
                        FontConfig = CreateFontConfig(),
                    };
                    label.ParagraphStyle.CharacterStyle.FontSize = 72;

                    var item = layout.Items.Add(label);
                    item.Left = 10;
                    item.Top = 10;
                    item.Right = 10;
                    item.Bottom = 10;
                }

                var container = new Ngs.UI.Widgets.Container()
                {
                    Layout = layout,
                };

                var window = new Window()
                {
                    ContentsView = container,
                };

                window.Close += (e, args) => {
                    Exit();
                };

                window.Visible = true;
            });

            this.Start();
        }
    }
}