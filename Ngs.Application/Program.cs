//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.IO;
using Ngs.Engine.UI;
using Ngs.Engine.UI.Theming;
using Ngs.Engine;
using Ngs.Engine.Presentation;
using Ngs.Engine.Canvas.Text;

namespace Ngs.Shell {
    sealed class Application : Ngs.Engine.Application {
        public static void Main(string[] args) {
            using (var thisApp = new Application()) {
                thisApp.Run();
            }
        }

        private Application() { }

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

        sealed class LinkLabel : Ngs.Engine.UI.Widgets.ButtonBase {
            public readonly Ngs.Engine.UI.Widgets.Label Label = new Ngs.Engine.UI.Widgets.Label();

            public LinkLabel() {
                UpdateTextColor();

                var layout = new AbsoluteLayout();
                var item = layout.Items.Add(Label);
                item.Left = 0;
                item.Top = 0;
                item.Right = 0;
                item.Bottom = 0;
                this.Layout = layout;
            }

            protected override void OnButtonStateUpdated(EventArgs e) => UpdateTextColor();

            void UpdateTextColor() {
                if (IsPressed && IsHovered) {
                    Label.TextColor = new Rgba(1, 0, 0, 1);
                } else if (IsHovered) {
                    Label.TextColor = new Rgba(1, 0.5f, 0, 1);
                } else {
                    Label.TextColor = new Rgba(0, 0, 1, 1);
                }
            }
        }

        sealed class Clock : Ngs.Engine.UI.Widgets.Label {
            System.Timers.Timer timer = new System.Timers.Timer(1000);

            public Clock() {
                timer.SynchronizingObject = this;
                timer.AutoReset = true;
                timer.Elapsed += delegate { Update(); };
                Update();
                timer.Start();
            }

            private void Update() {
                this.Text = DateTime.Now.ToString();
            }
        }

        sealed class MainView : Ngs.Engine.UI.Widgets.Container {
            protected override void RenderContents(RenderContext context) {
                Box2 boxBounds = LocalBounds.GetInflated(-40f);

                BoxRenderer.EmitBoxFill(context, LocalBounds, new Rgba(0.9f, 0.9f, 0.9f, 1.0f));

                BoxRenderer.EmitBoxFillShadow(context, boxBounds.GetTranslated(5, 10), 0.5f, 30f);
                BoxRenderer.EmitBoxFill(context, boxBounds, new Rgba(0.8f, 0.8f, 0.8f, 1.0f));

                base.RenderContents(context);
            }
        }

        private void Run() {
            Console.WriteLine("Displaying some window");

            this.UIQueue.Invoke(() => {
                var layout = new TableLayout()
                {
                    Padding = new Padding(60),
                };

                {
                    var label = new Ngs.Engine.UI.Widgets.Label()
                    {
                        Text = "Hello world",
                        TextColor = Rgba.Black,
                        FontConfig = CreateFontConfig(),
                    };
                    label.ParagraphStyle.CharacterStyle.FontSize = 72;

                    layout.Items.Add(label);
                }

                {
                    var label = new Clock()
                    {
                        TextColor = new Rgba(0.05f, 0.05f, 0.05f, 1),
                        FontConfig = CreateFontConfig(),
                    };
                    label.ParagraphStyle.CharacterStyle.FontSize = 16;

                    var item = layout.Items.Add(label);
                    item.Row = 1;
                }

                {
                    var label = new LinkLabel();
                    label.Label.Text = "This text is displayed using a label widget.";
                    label.Label.FontConfig = CreateFontConfig();
                    label.Label.ParagraphStyle.CharacterStyle.FontSize = 16;

                    var item = layout.Items.Add(label);
                    item.Row = 2;
                }

                var container = new MainView()
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