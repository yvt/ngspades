//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine.Presentation {
    /// <summary>
    /// Receives and handles workspace events.
    /// </summary>
    [Guid("44370e35-940c-4363-b11b-e5d93a87beb5")]
    public interface IWorkspaceListener : IUnknown {
        /// <summary>
        /// Retrieves the information about the application.
        /// </summary>
        /// <param name="name">The application name. Must not contain a null character
        /// (U+0000).</param>
        /// <param name="versionMajor">The major part of the version triplet. Must be in range
        /// <c>[0, 1023]</c>.</param>
        /// <param name="versionMinor">The minor part of the version triplet. Must be in range
        /// <c>[0, 1023]</c>.</param>
        /// <param name="versionRevision">The revision part of the version triplet. Must be in range
        /// <c>[0, 4095]</c>.</param>
        void GetApplicationInfo(out string name, out int versionMajor, out int versionMinor, out int versionRevision);
    }
}