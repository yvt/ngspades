//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using System.Security;
using Ngs.Interop;

namespace Ngs.Engine.Presentation {
    /// <summary>
    /// Contains some information about the client application.
    /// </summary>
    /// <remarks>
    /// The application information can be used, for example, by graphics
    /// driver vendors to optimize their drivers for a specific application.
    /// </remarks>
    public class ApplicationInfo {
        private string name = "Nightingales Application";
        private int versionMajor = 0;
        private int versionMinor = 0;
        private int versionRevision = 0;

        /// <summary>
        /// Constructs a new instance of <see cref="ApplicationInfo" />.
        /// </summary>
        public ApplicationInfo() {
        }

        /// <summary>
        /// Sets or retrieves the application name.
        /// </summary>
        /// <remarks>
        /// The application name must not contain a null character.
        /// </remarks>
        /// <returns>The application name.</returns>
        public string Name {
            get => name;
            set {
                if (value.Contains('\0')) {
                    throw new ArgumentException("The application name must not contain a null character.", nameof(value));
                }
                name = value;
            }
        }

        /// <summary>
        /// Sets or retrieves the major version number.
        /// </summary>
        /// <remarks>
        /// The major version number must be in range <c>[0, 1023]</c>.
        /// </remarks>
        /// <returns>The major version number.</returns>
        public int VersionMajor {
            get => versionMajor;
            set {
                if (value < 0 || value > 1023) {
                    throw new ArgumentException("The major version number must be in range [0, 1023].", nameof(value));
                }
                versionMajor = value;
            }
        }

        /// <summary>
        /// Sets or retrieves the minor version number.
        /// </summary>
        /// <remarks>
        /// The minor version number must be in range <c>[0, 1023]</c>.
        /// </remarks>
        /// <returns>The minor version number.</returns>
        public int VersionMinor {
            get => versionMinor;
            set {
                if (value < 0 || value > 1023) {
                    throw new ArgumentException("The minor version number must be in range [0, 1023].", nameof(value));
                }
                versionMinor = value;
            }
        }

        /// <summary>
        /// Sets or retrieves the revision version number.
        /// </summary>
        /// <remarks>
        /// The revision version number must be in range <c>[0, 4095]</c>.
        /// </remarks>
        /// <returns>The revision version number.</returns>
        public int VersionRevision {
            get => versionRevision;
            set {
                if (value < 0 || value > 4095) {
                    throw new ArgumentException("The revision version number must be in range [0, 4095].", nameof(value));
                }
                versionRevision = value;
            }
        }

    }
}