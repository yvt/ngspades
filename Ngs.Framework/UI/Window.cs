//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Security;
using Ngs.Engine.Native;

namespace Ngs.UI {
    // TODO: Event handling and window attributes

    /// <summary>
    /// Represents a window, the root-level component of an user interface.
    /// </summary>
    public class Window {
        Workspace workspace;
        INgsPFWindow pfWindow;

        /// <summary>
        /// Initializes a new instance of this class.
        /// </summary>
        [SecuritySafeCritical]
        public Window() {
            this.workspace = Application.EnsureInstance().Workspace;
            this.pfWindow = this.workspace.EngineWorkspace.Context.CreateWindow();
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
                // TODO: Reflect the measurement result to the window
                view.Arrange();
                view.Render();
            }
            this.pfWindow.Child = this.ContentsView?.MainPFLayer;
        }
    }
}
