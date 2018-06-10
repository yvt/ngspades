//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Security;
using Ngs.Engine.Presentation;

namespace Ngs.Engine {
    /// <summary>
    /// Represents an application based on the Nightingales engine.
    /// </summary>
    public class Application : IDisposable {
        private object constructLock = new object();

        /// <summary>
        /// Initializes a new instance of <see cref="Application" />.
        /// </summary>
        /// <remarks>
        /// The <see cref="Application" /> class is designed to implement the singleton pattern.
        /// This constructor automatically registers the new instance to the current application
        /// domain, and throws an exception if there exists the one that is already registered.
        /// </remarks>
        /// <exception name="InvalidOperationException">There already exists an instance of the
        /// <see cref="Application" /> class associated with the current application domain.
        /// </exception>
        [SecurityCritical]
        public Application() {
            lock (constructLock) {
                if (Instance != null) {
                    throw new InvalidOperationException(
                        "There already exists an application associated with the current application domain. ");
                }
                Instance = this;
            }

            this.UIQueue = new Ngs.Engine.Threading.DispatchQueue();
            this.Workspace = new UI.Workspace(this.ApplicationInfo, this.UIQueue);
            System.Threading.Thread.CurrentThread.Name = "Main";
        }

        /// <summary>
        /// Retrieves the instance of this class associated with the current application domain.
        /// </summary>
        /// <returns>The singleton instance of the <see cref="Application" /> class. <c>null</c>
        /// if one hasn't been created yet.</returns>
        public static Application Instance { get; private set; }

        internal static Application EnsureInstance() {
            var x = Instance;
            if (x == null) {
                throw new InvalidOperationException("The Application class must be instantiated before performing this operation.");
            }
            return x;
        }

        /// <summary>
        /// Retrieves the <see cref="ApplicationInfo" /> object containing information
        /// about this application.
        /// </summary>
        /// <remarks>
        /// <para>An <see cref="ApplicationInfo" /> object is passed to the video driver via the
        /// Nightingales compositing system. The main purpose of this value is to allow video
        /// drivers to perform optimizations specific to certain applications and game engines.
        /// </para>
        /// <para>This property is accessed by the constructor of the <see cref="Application" />
        /// class. This means that the constructor of a derived class has not yet been called at
        /// that point. You must keep that in mind when overriding this property.
        /// </para>
        /// <para>
        /// The default value is derived from the name and version of the entry assembly of the
        /// currently running <c>AppDomain</c>.
        /// </para>
        /// </remarks>
        /// <returns>The <see cref="ApplicationInfo" /> object.</returns>
        protected virtual ApplicationInfo ApplicationInfo {
            get {
                var entryAssembly = System.Reflection.Assembly.GetEntryAssembly();
                var name = entryAssembly.GetName();
                return new ApplicationInfo()
                {
                    Name = name.Name,
                    VersionMajor = Math.Clamp(name.Version.Major, 0, 1023),
                    VersionMinor = Math.Clamp(name.Version.Minor, 0, 1023),
                    VersionRevision = Math.Clamp(name.Version.Revision, 0, 4095),
                };
            }
        }

        /// <summary>
        /// Retrieves the global <see cref="UI.Workspace" /> object associated with this application.
        /// </summary>
        /// <returns>The <see cref="UI.Workspace" /> object.</returns>
        public UI.Workspace Workspace { get; }

        /// <summary>
        /// Retrieves the dispatch queue used for the user interface event handling.
        /// </summary>
        /// <remarks>
        /// The thread associated with the queue does not necessary match the main thread as defined
        /// by the operating system.
        /// </remarks>
        /// <returns>A dispatch queue.</returns>
        public Ngs.Engine.Threading.DispatchQueue UIQueue { get; }

        /// <summary>
        /// Starts the main loop.
        /// </summary>
        /// <remarks>
        /// <para>This method returns only when the <see cref="Exit" /> method is called from
        /// another thread.</para>
        /// <para>This method only can be called by a thread that created this instance.</para>
        /// </remarks>
        public void Start() {
            // Make sure changes are reflected to the compositor
            // (`DispatchQueue` strictly follows FIFO, so this ensures that all
            // pending works are completed)
            UIQueue.Invoke(() => { });

            this.Workspace.EngineWorkspace.Start();
        }

        /// <summary>
        /// Releases the resources used by <see cref="Application" />.
        /// </summary>
        public void Dispose() => UIQueue.Exit();

        /// <summary>
        /// Causes the main loop to terminate.
        /// </summary>
        public void Exit() {
            this.Workspace.EngineWorkspace.Exit();
        }
    }
}
