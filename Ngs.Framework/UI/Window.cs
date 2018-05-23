//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.ComponentModel;
using System.Security;
using System.Numerics;
using Ngs.Engine.Native;
using Ngs.Engine.Presentation;
using Ngs.Utils;

namespace Ngs.UI {
    // TODO: Event handling and window attributes

    /// <summary>
    /// Represents a window, the root-level component of an user interface.
    /// </summary>
    public class Window {
        Workspace workspace;
        INgsPFWindow pfWindow;

        /// <summary>
        /// Is the window already materialized?
        /// </summary>
        bool materialized;

        /// <summary>
        /// The size of the client region. <c>null</c> if not computed or specified yet
        /// </summary>
        Vector2? size;

        /// <summary>
        /// Should be the value of <see cref="size" /> pushed to the compositor?
        /// </summary>
        bool shouldPushSize;

        /// <summary>
        /// Initializes a new instance of this class.
        /// </summary>
        [SecuritySafeCritical]
        public Window() {
            this.workspace = Application.EnsureInstance().Workspace;
            this.pfWindow = this.workspace.EngineWorkspace.Context.CreateWindow();
            this.pfWindow.Listener = new Listener(this);

            // Provide a default value for `Title`
            this.title = Application.Instance.GetType().Assembly.GetName().Name;
        }

        readonly WindowContentsLayout dummyLayout = new WindowContentsLayout();

        /// <summary>
        /// Sets or retrieves the contents (root) view of this window.
        /// </summary>
        /// <returns>The contents view of this window.</returns>
        public View ContentsView {
            get => dummyLayout.ContentsView;
            set {
                dummyLayout.ContentsView = value;
                this.workspace.SetNeedsUpdate();
            }
        }

        bool borderless;

        /// <summary>
        /// Sets or retrieves a flag indicating whether the window has a border provided by the
        /// window system.
        /// </summary>
        /// <returns><c>true</c> is the window is borderless; otherwise, <c>false</c>.</returns>
        public bool Borderless {
            get => borderless;
            set {
                if (materialized) {
                    throw new InvalidOperationException("The window is already materialized.");
                }
                borderless = value;
            }
        }

        string title;
        bool shouldPushTitle = true;

        /// <summary>
        /// Sets or retrieves the window title.
        /// </summary>
        /// <returns>The window title.</returns>
        public string Title {
            get => title;
            set {
                title = value ?? "";
                shouldPushTitle = true;
            }
        }

        /// <summary>
        /// Sets or retrieves a flag indicating whether this window is displayed on the screen.
        /// </summary>
        /// <returns><c>true</c> if the window is visible; otherwise, <c>false</c>.</returns>
        public bool Visible {
            get => this.workspace.IsWindowVisible(this);
            set => this.workspace.SetWindowVisible(this, value);
        }

        internal INgsPFWindow PFWindow { get => pfWindow; }

        internal void Render() {
            // TODO: Minimize the update
            if (this.ContentsView is View view) {
                view.BeforeLayout();
                view.Measure();

                if (!size.HasValue) {
                    // The window size is not specified yet. Automatically derive it from the
                    // measurement result.
                    size = view.Measurement.PreferredSize;
                    shouldPushSize = true;
                } else {
                    // Limit the size according to the measurement result.
                    var measurement = view.Measurement;
                    var newSize = Vector2.Clamp(size.Value,
                        measurement.MinimumSize,
                        measurement.MaximumSize);
                    if (newSize != size) {
                        size = newSize;
                        shouldPushSize = true;
                    }
                }

                view.Bounds = new Box2(Vector2.Zero, size.Value);
                view.Arrange();
                view.Render();
            } else {
                // Ditto, but without contents
                if (!size.HasValue) {
                    size = Vector2.Zero;
                    shouldPushSize = true;
                }
            }

            if (!this.materialized) {
                // Window flags can be set only if the window is not materialized yet.
                // (This is a restriction imposed by the `winit` library.)
                WindowFlags flags = WindowFlags.Resizable;

                if (this.Borderless) {
                    flags |= WindowFlags.Borderless | WindowFlags.Transparent;
                }

                // Deny resizing at all if the root view has the maximum size
                if (this.ContentsView is View view2) {
                    var maxSize = view2.Measurement.MaximumSize;
                    if (!float.IsInfinity(maxSize.X) || !float.IsInfinity(maxSize.Y)) {
                        flags &= ~WindowFlags.Resizable;
                    }
                }

                this.pfWindow.Flags = flags;
            }

            if (shouldPushSize) {
                this.pfWindow.Size = size.Value;
                shouldPushSize = false;
            }

            if (shouldPushTitle) {
                this.pfWindow.Title = title;
                shouldPushTitle = false;
            }

            // TODO: Repond to the resize event and re-render accordingly

            this.pfWindow.Child = this.ContentsView?.MainPFLayer;
            this.materialized = true;
        }

        sealed class Listener : Ngs.Interop.ComClass<Listener>, INgsPFWindowListener {
            // Break the circular reference that cannot be handled by GC
            // (`Window` -> native `Window` -> `Listener` -> `Window`)
            readonly WeakReference<Window> window;

            public Listener(Window window) => this.window = new WeakReference<Window>(window);

            /// <summary>
            /// Retrieves a strong reference to the owning window, and calls a given delegate
            /// with it on the main thread.
            /// </summary>
            private void InvokeOnWindow(Action<Window> action) {
                if (!this.window.TryGetTarget(out var window)) {
                    return;
                }

                window.workspace.DispatchQueue.InvokeAsync(() => {
                    action(window);
                });
            }

            public void Close() {
                InvokeOnWindow((window) => {
                    var e = new CancelEventArgs();
                    window.OnClose(e);
                    if (!e.Cancel) {
                        window.Visible = false;
                    }
                });
            }

            public void Focused(bool focused) {
                // TODO: Handle focus events
            }

            public void KeyboardInput(string virtualKeyCode, bool pressed, KeyModifierFlags modifier) {
                // TODO: Handle keyboard events
            }

            public void MouseButton(MousePosition position, byte button, bool pressed) {
                // TODO: Handle mouse events
            }

            public void MouseLeave() {
                // TODO: Handle mouse events
            }

            public void MouseMotion(MousePosition position) {
                // TODO: Handle mouse events
            }

            public void Moved(Vector2 position) {
            }

            public void Resized(Vector2 size) {
                InvokeOnWindow((window) => {
                    window.size = size;
                    window.workspace.SetNeedsUpdate();
                });
            }
        }

        /// <summary>
        /// Occurs when a user clicks the close button of this window.
        /// </summary>
        public event EventHandler<CancelEventArgs> Close;

        /// <summary>
        /// Called when a user clicks the close button of this window.
        /// </summary>
        protected virtual void OnClose(CancelEventArgs e) {
            Close?.Invoke(this, e);
        }
    }
}
