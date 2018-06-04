//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using System.Security;
using Ngs.Interop;
using Ngs.Engine.Native;

namespace Ngs.Engine.Presentation {
    /// <summary>
    /// An interface to the window system of the target system.
    /// </summary>
    public class Workspace {
        INgsPFWorkspace nativeWorkspace;

        IUnknown windows;

        private sealed class Listener : ComClass<Listener>, INgsPFWorkspaceListener {
            public ApplicationInfo applicationInfo;

            public void GetApplicationInfo(out string name, out int versionMajor, out int versionMinor, out int versionRevision) {
                name = applicationInfo.Name;
                versionMajor = applicationInfo.VersionMajor;
                versionMinor = applicationInfo.VersionMinor;
                versionRevision = applicationInfo.VersionRevision;
            }
        }

        /// <summary>
        /// Constructs a new instance of <see cref="Workspace" />.
        /// </summary>
        /// <remarks>
        /// The calling thread will be registered as the main thread by the underlying
        /// implementation.
        /// </remarks>
        /// <param name="applicationInfo">The information about the client application.</param>
        [SecuritySafeCritical]
        public Workspace(ApplicationInfo applicationInfo) {
            var listener = new Listener()
            {
                applicationInfo = applicationInfo
            };
            nativeWorkspace = EngineInstance.NativeEngine.CreateWorkspace(listener);
        }

        /// <summary>
        /// Retrieves the underlying native workspace object.
        /// </summary>
        /// <returns>The underlying native workspace object.</returns>
        public INgsPFWorkspace NativeWorkspace {
            [SecurityCritical]
            get => nativeWorkspace;
        }

        /// <summary>
        /// Retrieves the associated presentation context.
        /// </summary>
        /// <returns>The presentation context managed by this workspace.</returns>
        public INgsPFContext Context {
            [SecurityCritical]
            get => nativeWorkspace.Context;
        }

        /// <summary>
        /// Sets the window node.
        /// </summary>
        /// <returns>
        /// The top-level window node (or a group of window nodes) of the workspace, or the <c>null</c> value.
        /// </returns>
        // TODO: Use a safe wrapper
        public IUnknown Windows {
            set {
                nativeWorkspace.Windows = value;
                this.windows = value;
            }
            get => this.windows;
        }

        /// <summary>
        /// Enter the main loop.
        /// </summary>
        /// <remarks>
        /// <para>This method returns only if an exit request was made from another thread by
        /// calling the <see cref="Exit" /> method.</para>
        /// <para>This method only can be called by a main thread (the thread that created this
        /// workspace).</para>
        /// </remarks>
        public void Start() => nativeWorkspace.Start();

        /// <summary>
        /// Causes the main loop to terminate.
        /// </summary>
        /// <remarks>
        /// The exit request is a part of presentation properties. You must call
        /// <see cref="INgsPFContext.CommitFrame" /> afterward for it to take effect.
        /// </remarks>
        public void Exit() => nativeWorkspace.Exit();
    }
}