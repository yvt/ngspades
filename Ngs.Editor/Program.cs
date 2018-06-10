//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.IO;
using Ngs.Engine;
using Ngs.Engine.UI;
using Ngs.Engine.Presentation;
using Ngs.Engine.Canvas.Text;

namespace Ngs.Editor {
    sealed class Application : Ngs.Engine.Application {
        public static void Main(string[] args) {
            using (var thisApp = new Application()) {
                thisApp.Run();
            }
        }

        private Application() { }

        protected override ApplicationInfo ApplicationInfo {
            get {
                var info = base.ApplicationInfo;
                info.Name = "Nightingales Editor";
                return info;
            }
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

        private void Run() {
            this.UIQueue.Invoke(() => {
                var layout = new TableLayout()
                {
                    Padding = new Padding(10),
                };

                {
                    var label = new Ngs.Engine.UI.Widgets.Label()
                    {
                        Text = "Hello world",
                        TextColor = Rgba.White,
                        FontConfig = CreateFontConfig(),
                    };
                    label.ParagraphStyle.CharacterStyle.FontSize = 72;

                    layout.Items.Add(label);
                }

                var container = new Ngs.Engine.UI.Widgets.Container()
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