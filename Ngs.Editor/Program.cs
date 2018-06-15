//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.IO;
using Ngs.Engine;
using Ngs.Engine.UI;
using Ngs.Engine.UI.Theming;
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

        private static void LoadFonts(FontManager fontManager) {
            byte[] ReadAllBytes(Stream s) {
                using (var ms = new MemoryStream()) {
                    s.CopyTo(ms);
                    return ms.ToArray();
                }
            }

            var assembly = typeof(Application).Assembly;
            var fontData = ReadAllBytes(assembly.GetManifestResourceStream("Fonts.DDin"));
            var font = new Font(fontData);

            fontManager.AddFontFace(font.FontFaces[0], "D-DIN", FontStyle.Normal, 300);
        }

        private void Run() {
            this.UIQueue.Invoke(() => {
                LoadFonts(FontManager.Default);

                var layout = new TableLayout()
                {
                    Padding = new Padding(10),
                };

                {
                    var label = new Ngs.Engine.UI.Widgets.Label()
                    {
                        Text = "Hello world",
                        TextColor = Rgba.White,
                    };

                    var paraStyle = FontManager.Default.DefaultParagraphStyle.Clone();
                    paraStyle.CharacterStyle.FontSize = 72;
                    label.ParagraphStyle = paraStyle;

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