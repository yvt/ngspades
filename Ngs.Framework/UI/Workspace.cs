//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using System.Collections.Generic;
using Ngs.Engine.Presentation;
using Ngs.Engine.Native;
using Ngs.Threading;

namespace Ngs.UI {
    /// <summary>
    /// Manages the windows owned by an application.
    /// </summary>
    public class Workspace {
        /// <summary>
        /// Initializes a new instance of this class.
        /// </summary>
        /// <remarks>
        /// The calling thread will be registered as the main thread by the underlying
        /// implementation.
        /// </remarks>
        internal Workspace(ApplicationInfo applicationInfo, DispatchQueue uiQueue) {
            this.EngineWorkspace = new Ngs.Engine.Presentation.Workspace(applicationInfo);
            this.DispatchQueue = uiQueue;
        }

        /// <summary>
        /// Retrieves the underlying <see cref="Ngs.Engine.Presentation.Workspace"/> object.
        /// </summary>
        /// <returns>The underlying <see cref="Ngs.Engine.Presentation.Workspace"/> object.</returns>
        public Ngs.Engine.Presentation.Workspace EngineWorkspace { get; }

        /// <summary>
        /// Retrieves the dispatch queue used for the user interface event handling.
        /// </summary>
        /// <returns>A dispatch queue.</returns>
        public Ngs.Threading.DispatchQueue DispatchQueue { get; }

        Dictionary<Window, object> windows = new Dictionary<Window, object>();

        internal bool IsWindowVisible(Window window) {
            DispatchQueue.VerifyAccess();
            return windows.ContainsKey(window);
        }

        internal void SetWindowVisible(Window window, bool visible) {
            DispatchQueue.VerifyAccess();
            if (visible) {
                windows[window] = null;
            } else {
                windows.Remove(window);
            }
            SetNeedsUpdate();
        }

        bool needsUpdate;

        /// <summary>
        /// Marks that the contents of some windows should be updated.
        /// </summary>
        /// <remarks>
        /// TODO: Make the update more efficient by skipping elements that did not change
        /// </remarks>
        internal void SetNeedsUpdate() {
            DispatchQueue.VerifyAccess();
            if (needsUpdate) {
                return;
            }
            needsUpdate = true;
            DispatchQueue.InvokeAsync(Update);
        }

        void Update() {
            needsUpdate = false;

            var pfWorkspace = this.EngineWorkspace;
            INgsPFContext pfContext = pfWorkspace.Context;

            pfContext.Lock();

            foreach (var window in windows.Keys) {
                window.Render();
            }

            // TODO: Optimize the update here
            var group = pfContext.CreateNodeGroup();
            foreach (var window in windows.Keys) {
                group.Insert(window.PFWindow);
            }
            pfWorkspace.Windows = group;

            pfContext.Unlock();
            pfContext.CommitFrame();
        }
    }
}
