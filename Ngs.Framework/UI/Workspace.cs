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

        /// <summary>
        /// Called when an unhandled exception was thrown by a component.
        /// </summary>
        /// <remarks>
        /// This method either returns normally (the exception is ignored and/or reported somehow)
        /// or never returns (terminates the application).
        /// </remarks>
        /// <param name="exception">The exception that was thrown.</param>
        internal void OnUnhandledException(Exception exception) {
            // TODO: Provide more options for error handling
            Console.WriteLine($"Aborting due to an unhandled exception.: {exception.Message}");

            // Cause the app domain to terminate by rethrowing the exception in another thread.
            //
            // (Rethrowing the exception is not allowed here as per this method's specification.
            // For example, it may be translated into a COM error, which is passed to the native
            // code that doesn't know how to handle such an error and just aborts, which complicates
            // the debugging further.)
            var thread = new System.Threading.Thread(() => {
                throw new Exception("An unhandled exception was thrown by a component.", exception);
            });
            thread.Start();
            thread.Join();
            System.Environment.Exit(1);
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

        /// <summary>
        /// Processes the requests to change the currently focused views and ensures the focus
        /// event handlers are called accordingly.
        /// </summary>
        internal void UpdateFocus() {
            // TODO: This method is not reentrant. Handle the recursive call

            // We must ensure the focus handlers are called in the order expected by components,
            // even if components were moved between windows.
            // To ensure the consistent ordering, the update is broken into two phases.
            foreach (var window in windows.Keys) {
                window.UpdateFocusEarly();
            }
            foreach (var window in windows.Keys) {
                window.UpdateFocusLate();
            }
        }

        void Update() {
            // TODO: `UpdateFocus` has to be called only in the following situations:
            //       (a) The setter of a property that modifies the focus state directly was
            //           called and must apply the changes immediately.
            //       (b) An operation that potentially modifies the view hierarchy was performed.
            //           The assignment to the `View.Layout` property is an example of this case.
            UpdateFocus();

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
