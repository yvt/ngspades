//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Diagnostics;
using System.ComponentModel;
using System.Security;
using System.Numerics;
using System.Collections.Generic;
using Ngs.Engine.Native;
using Ngs.Engine.Presentation;
using Ngs.Engine;

namespace Ngs.Engine.UI {
    // TODO: Event handling and window attributes

    /// <summary>
    /// Represents a window, the root-level component of an user interface.
    /// </summary>
    public class Window : ISynchronizeInvoke {
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

            this.dummyLayout = new WindowContentsLayout(this);

            this.systemMouseWindow = Input.SystemMouseDevice.Instance.CreateManagerForWindow(this);

            // Provide a default value for `Title`
            this.title = Application.Instance.GetType().Assembly.GetName().Name;
        }

        readonly WindowContentsLayout dummyLayout;

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

        internal View MouseHitTest(Vector2 point) {
            if (this.ContentsView is View view) {
                return view.MouseHitTest(point);
            } else {
                return null;
            }
        }

        #region Focus management

        /// <summary>
        /// The currently focused view. Can temporarily point an invalid view.
        /// </summary>
        View focusedView;

        /// <summary>
        /// The next view to be focused. The value of this property will be finalized
        /// (i.e. copied to <see cref="focusedView" />) when the focus state is updated.
        /// </summary>
        View newFocusedView;

        List<View> focusedViewPath = new List<View>();
        List<View> newFocusedViewPath = new List<View>();
        int focusedViewPathCommonPrefixLength;

        /// <summary>
        /// The currently focused view.
        /// </summary>
        internal View FocusedView {
            get {
                var view = focusedView;
                if (view == null || !view.CanGetFocus) {
                    return null;
                } else {
                    return view;
                }
            }
            set {
                if (value == focusedView) {
                    return;
                }
                newFocusedView = value;
                workspace.UpdateFocus();
            }
        }

        internal void UpdateFocusEarly() {
            var focusedViewPath = this.focusedViewPath;
            var focusedView = this.focusedView;
            var newFocusedViewPath = this.newFocusedViewPath;
            var newFocusedView = this.newFocusedView;

            newFocusedViewPath.Clear();

            // Reject an invalid view
            if (newFocusedView != null &&
                (newFocusedView.Window != this || !newFocusedView.CanGetFocus)) {
                this.newFocusedView = newFocusedView = null;
            }

            // Compute the path
            if (newFocusedView != null) {
                for (var view = newFocusedView; view != null; view = view.Superview) {
                    newFocusedViewPath.Add(view);
                }
                newFocusedViewPath.Reverse();
            }

            // Compute the common prefix length
            int commonPrefixLength = 0;
            for (int limit = Math.Min(focusedViewPath.Count, newFocusedViewPath.Count);
                commonPrefixLength < limit; ++commonPrefixLength) {
                if (focusedViewPath[commonPrefixLength] != newFocusedViewPath[commonPrefixLength]) {
                    break;
                }
            }
            this.focusedViewPathCommonPrefixLength = commonPrefixLength;

            // Call handlers
            this.focusedView = newFocusedView;
            for (int i = focusedViewPath.Count; i > commonPrefixLength; --i) {
                if (i == focusedViewPath.Count) {
                    focusedViewPath[i - 1].OnLostFocus(EventArgs.Empty);
                }
                focusedViewPath[i - 1].OnLeave(EventArgs.Empty);
            }
            focusedViewPath.Clear();
        }

        internal void UpdateFocusLate() {
            var focusedViewPath = this.focusedViewPath;
            var newFocusedViewPath = this.newFocusedViewPath;
            int commonPrefixLength = this.focusedViewPathCommonPrefixLength;

            // Call handlers
            for (int i = commonPrefixLength; i < newFocusedViewPath.Count; ++i) {
                newFocusedViewPath[i].OnEnter(EventArgs.Empty);
                if (i == newFocusedViewPath.Count - 1) {
                    newFocusedViewPath[i].OnGotFocus(EventArgs.Empty);
                }
            }

            // Swap
            this.focusedViewPath = newFocusedViewPath;
            this.newFocusedViewPath = focusedViewPath;
        }

        #endregion

        #region Mouse input handling

        private Input.SystemMouseDevice.WindowManager systemMouseWindow;

        // FIXME: This part is very similar to "Focus management".
        //        (Except that this one is more fragile against structural changes).
        //        Should be deduped!
        View hotView;

        View HotView {
            get => hotView;
            set {
                if (value == hotView) {
                    return;
                }

                // Compute the path
                var path = new List<View>();
                var newPath = new List<View>();
                if (value != null) {
                    for (var view = value; view != null; view = view.Superview) {
                        newPath.Add(view);
                    }
                    newPath.Reverse();
                }
                if (hotView != null) {
                    for (var view = hotView; view != null; view = view.Superview) {
                        path.Add(view);
                    }
                    path.Reverse();
                }

                // Compute the common prefix length
                int commonPrefixLength = 0;
                for (int limit = Math.Min(path.Count, newPath.Count);
                    commonPrefixLength < limit; ++commonPrefixLength) {
                    if (path[commonPrefixLength] != newPath[commonPrefixLength]) {
                        break;
                    }
                }

                // Call handlers
                for (int i = path.Count; i > commonPrefixLength; --i) {
                    path[i - 1].OnMouseLeave(EventArgs.Empty);
                }
                for (int i = commonPrefixLength; i < newPath.Count; ++i) {
                    newPath[i].OnMouseEnter(EventArgs.Empty);
                }

                hotView = value;
            }
        }

        internal void UpdateMouse() {
            this.systemMouseWindow.Update();
            if (hotView?.Window != this) {
                hotView = null;
            }
        }

        void UpdateHotTracking(MousePosition position) {
            HotView = MouseHitTest(position.Client);
        }

        void HandleMouseLeave() {
            HotView = null;
        }

        #endregion

        sealed class Listener : Ngs.Interop.ComClass<Listener>, INgsPFWindowListener {
            // Break the circular reference that cannot be handled by GC
            // (`Window` -> native `Window` -> `Listener` -> `Window`)
            readonly WeakReference<Window> window;

            public Listener(Window window) => this.window = new WeakReference<Window>(window);

            sealed class Trampoline {
                public Action<Window> action;
                public Window window;

                // Do not execute the `catch` clause automatically when Just My Code is enabled
                [DebuggerNonUserCode]
                public void Invoke() {
                    try {
                        action(window);
                    } catch (Exception e) {
                        window.workspace.OnUnhandledException(e);
                    }
                }
            }

            /// <summary>
            /// Retrieves a strong reference to the owning window, and calls a given delegate
            /// with it on the main thread.
            /// </summary>
            private void InvokeOnWindow(Action<Window> action) {
                if (!this.window.TryGetTarget(out var window)) {
                    return;
                }

                window.workspace.DispatchQueue.InvokeAsync(new Trampoline()
                {
                    action = action,
                    window = window,
                }.Invoke);
            }

            void INgsPFWindowListener.Close() {
                InvokeOnWindow((window) => {
                    var e = new CancelEventArgs();
                    window.OnClose(e);
                    if (!e.Cancel) {
                        window.Visible = false;
                    }
                });
            }

            void INgsPFWindowListener.Focused(bool focused) {
                // TODO: Handle focus events
            }

            void INgsPFWindowListener.KeyboardInput(string virtualKeyCode, bool pressed, KeyModifierFlags modifier) {
                // TODO: Handle keyboard events
            }

            void INgsPFWindowListener.MouseButton(MousePosition position, byte button, bool pressed) {
                InvokeOnWindow((window) => {
                    window.systemMouseWindow.HandleMouseButton(position, button, pressed);

                    if (!window.systemMouseWindow.HasAnyButtonsPressed) {
                        window.UpdateHotTracking(position);
                    }
                });
            }

            void INgsPFWindowListener.MouseLeave() {
                InvokeOnWindow((window) => {
                    window.HandleMouseLeave();
                });
            }

            void INgsPFWindowListener.MouseMotion(MousePosition position) {
                InvokeOnWindow((window) => {
                    window.systemMouseWindow.HandleMouseMotion(position);

                    if (!window.systemMouseWindow.HasAnyButtonsPressed) {
                        window.UpdateHotTracking(position);
                    }
                });
            }

            void INgsPFWindowListener.Moved(Vector2 position) {
            }

            void INgsPFWindowListener.Resized(Vector2 size) {
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

        #region ISynchronizeInvoke implementation
        /// <summary>
        /// Implemenets <see cref="ISynchronizeInvoke.InvokeRequired" />.
        /// </summary>
        public bool InvokeRequired => workspace.DispatchQueue.InvokeRequired;

        /// <summary>
        /// Implemenets <see cref="ISynchronizeInvoke.BeginInvoke(Delegate, object[])" />.
        /// </summary>
        public IAsyncResult BeginInvoke(Delegate method, object[] args) =>
            workspace.DispatchQueue.BeginInvoke(method, args);

        /// <summary>
        /// Implemenets <see cref="ISynchronizeInvoke.EndInvoke(IAsyncResult)" />.
        /// </summary>
        public object EndInvoke(IAsyncResult result) =>
            workspace.DispatchQueue.EndInvoke(result);

        /// <summary>
        /// Implements <see cref="ISynchronizeInvoke.Invoke(Delegate, object[])" />.
        /// </summary>
        public object Invoke(Delegate method, object[] args) =>
            workspace.DispatchQueue.Invoke(method, args);
        #endregion
    }
}
