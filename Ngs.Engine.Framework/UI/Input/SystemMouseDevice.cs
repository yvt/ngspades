//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Collections.Generic;
using Ngs.Engine.Presentation;

namespace Ngs.Engine.UI.Input {
    /// <summary>
    /// Represents a system mouse device.
    /// </summary>
    internal sealed class SystemMouseDevice : MouseDevice {
        private SystemMouseDevice() {
        }

        /// <summary>
        /// Retrieves the singleton instance of this class.
        /// </summary>
        public static readonly SystemMouseDevice Instance = new SystemMouseDevice();

        public sealed class WindowManager : IDisposable {
            readonly Window window;
            /// <summary>
            /// The set of currently pressed mouse buttons. For each pair <c>(k, v)</c>, <c>k</c>
            /// represents the mouse button code passed by the engine and <c>v</c> stores the value
            /// of <c>SystemMouseButton.GetButtonFromPFValue(k)</c>.
            /// </summary>
            readonly Dictionary<byte, SystemMouseButton> pressedButtons =
                new Dictionary<byte, SystemMouseButton>();
            View.MouseCapture mouseCapture;

            // I wish there were `pub(super)` in C♯...
            internal WindowManager(Window window) {
                this.window = window;
            }

            public void Dispose() {
                this.mouseCapture?.Dispose();
                this.mouseCapture = null;
            }

            public bool HasAnyButtonsPressed => this.pressedButtons.Count > 0;

            private WindowPoint TranslatePoint(MousePosition position) =>
                new WindowPoint(this.window, position.Client);

            public void HandleMouseButton(MousePosition position, byte button, bool pressed) {
                if (pressed) {
                    if (this.pressedButtons.ContainsKey(button)) {
                        // Nani the blyat
                        return;
                    }

                    var buttonObject = SystemMouseButton.GetButtonFromPFValue(button);
                    this.pressedButtons.Add(button, buttonObject);

                    // If no buttons were pressed previously, try to acquire the mouse capture on
                    // the view below the mouse pointer.
                    if (this.pressedButtons.Count == 1) {
                        // Find the view below the mouse pointer. This can return `null`.
                        var view = this.window.MouseHitTest(position.Client);

                        // Capture the view. This can also return `null` on failure.
                        this.mouseCapture = view?.AcquireMouseCapture(Instance);
                    }

                    if (this.mouseCapture is View.MouseCapture capture) {
                        // Send the event to the captured view
                        capture.MouseDown(new MouseButtonEventArgs(
                            TranslatePoint(position), buttonObject));
                    }
                } else {
                    if (!this.pressedButtons.TryGetValue(button, out var buttonObject)) {
                        // Nani the blyat
                        return;
                    }
                    this.pressedButtons.Remove(button);

                    if (this.mouseCapture is View.MouseCapture capture) {
                        // Send the event to the captured view
                        capture.MouseUp(new MouseButtonEventArgs(
                            TranslatePoint(position), buttonObject));

                        // If it was the last button held down, then release the mouse capture
                        if (this.pressedButtons.Count == 0) {
                            capture.Dispose();
                            this.mouseCapture = null;
                        }
                    }
                }
            }

            /// <summary>
            /// Handles a mouse motion event.
            /// </summary>
            /// <param name="position">The position of the mouse pointer.</param>
            public void HandleMouseMotion(MousePosition position) {
                var e = new MouseEventArgs(TranslatePoint(position));

                if (this.mouseCapture is View.MouseCapture capture) {
                    // Send the mouse event to the currently captured view
                    capture.MouseMove(e);
                } else {
                    // Find the view below the mouse pointer. This can return `null`.
                    var view = this.window.MouseHitTest(position.Client);

                    // Capture the view temporarily and send the event.
                    // This can also return `null` on failure.
                    using (var ephemeralCapture = view?.AcquireMouseCapture(Instance)) {
                        ephemeralCapture?.MouseMove(e);
                    }
                }
            }

            /// <summary>
            /// Checks the validity of mouse capture and releases it if necessary.
            /// </summary>
            public void Update() {
                if (this.mouseCapture is View.MouseCapture capture) {
                    if (capture.View.Window != this.window) {
                        // This view is no longer in the same window -- release the mouse capture
                        capture.Dispose();
                        this.mouseCapture = null;
                    }
                }
            }
        }

        public WindowManager CreateManagerForWindow(Window window) {
            return new WindowManager(window);
        }
    }
}
