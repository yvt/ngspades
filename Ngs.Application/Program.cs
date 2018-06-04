//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.IO;
using Ngs.UI;
using Ngs.Engine;
using Ngs.Engine.Presentation;
using Ngs.Engine.Canvas.Text;

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

        sealed class LinkLabel : Ngs.UI.Widgets.Label {
            bool hot, pressed;

            public LinkLabel() {
                EnableMouseTracking = true;
                UpdateTextColor();
            }

            void UpdateTextColor() {
                if (pressed) {
                    TextColor = new Rgba(1, 0, 0, 1);
                } else if (hot) {
                    TextColor = new Rgba(1, 0.5f, 0, 1);
                } else {
                    TextColor = new Rgba(0, 1, 1, 1);
                }
            }

            protected override void OnMouseDown(Ngs.UI.Input.MouseButtonEventArgs e) {
                if (e.Button.Type == Ngs.UI.Input.MouseButtonType.Left) {
                    pressed = true;
                    UpdateTextColor();
                }
            }
            protected override void OnMouseUp(Ngs.UI.Input.MouseButtonEventArgs e) {
                if (e.Button.Type == Ngs.UI.Input.MouseButtonType.Left) {
                    pressed = false;
                    UpdateTextColor();
                }
            }
            protected override void OnMouseCancel(Ngs.UI.Input.MouseButtonEventArgs e) {
                if (e.Button.Type == Ngs.UI.Input.MouseButtonType.Left) {
                    pressed = false;
                    UpdateTextColor();
                }
            }

            protected override void OnMouseEnter(EventArgs e) {
                hot = true;
                UpdateTextColor();
            }

            protected override void OnMouseLeave(EventArgs e) {
                hot = false;
                UpdateTextColor();
            }
        }

        sealed class Clock : Ngs.UI.Widgets.Label {
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
                var layout = new TableLayout()
                {
                    Padding = new Padding(10),
                };

                {
                    var label = new Ngs.UI.Widgets.Label()
                    {
                        Text = "Hello world",
                        TextColor = Rgba.White,
                        FontConfig = CreateFontConfig(),
                    };
                    label.ParagraphStyle.CharacterStyle.FontSize = 72;

                    layout.Items.Add(label);
                }

                {
                    var label = new Clock()
                    {
                        TextColor = new Rgba(0.5f, 0.5f, 0.5f, 1),
                        FontConfig = CreateFontConfig(),
                    };
                    label.ParagraphStyle.CharacterStyle.FontSize = 16;

                    var item = layout.Items.Add(label);
                    item.Row = 1;
                }

                {
                    var label = new LinkLabel()
                    {
                        Text = "This text is displayed using a label widget.",
                        FontConfig = CreateFontConfig(),
                    };
                    label.ParagraphStyle.CharacterStyle.FontSize = 16;

                    var item = layout.Items.Add(label);
                    item.Row = 2;
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