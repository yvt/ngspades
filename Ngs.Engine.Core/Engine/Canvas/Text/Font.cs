//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Security;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Utils;
using Ngs.Engine.Native;
using System.Collections.Generic;
using System.Collections;

namespace Ngs.Engine.Canvas.Text {
    /// <summary>
    /// A font file loaded into the memory, containing one or more font faces.
    /// </summary>
    /// <remarks>
    /// <para>This class is a wrapper of <see cref="INgsPFFont" />.</para>
    /// </remarks>
    public class Font {
        private INgsPFFont nativeObject;

        internal Font(INgsPFFont nativeObject) {
            this.nativeObject = nativeObject;
        }

        /// <summary>
        /// Creates a font object.
        /// </summary>
        /// <param name="bytes">A TrueType or OpenType binary blob.</param>
        /// <returns>A newly created font object.</returns>
        [SecuritySafeCritical]
        public unsafe Font(Span<byte> bytes) {
            fixed (byte* ptr = &bytes[0]) {
                nativeObject = EngineInstance.NativeEngine.FontFactory
                    .CreateFont((IntPtr)ptr, bytes.Length);
            }
        }

        internal INgsPFFont NativeFont {
            [SecurityCritical]
            get => nativeObject;
        }

        /// <summary>
        /// Gets the collection of font faces in a font object.
        /// </summary>
        /// <returns>The collection of font faces in a font object.</returns>
        public FontFaceCollection FontFaces {
            get => new FontFaceCollection()
            {
                nativeFont = nativeObject,
                count = nativeObject.NumFontFaces,
            };
        }

        /// <summary>
        /// A collection of font faces in a font.
        /// </summary>
        public struct FontFaceCollection : IReadOnlyList<FontFace> {
            internal INgsPFFont nativeFont;
            internal int count;

            /// <summary>
            /// Gets the number of font faces.
            /// </summary>
            /// <returns>The number of font faces.</returns>
            public int Count { get => count; }

            /// <summary>
            /// Gets the element at the specified index.
            /// </summary>
            /// <param name="index">The zero-vased index.</param>
            /// <returns>The element at the specified index.</returns>
            public FontFace this[int index] {
                get {
                    if (index < 0 || index >= count) {
                        throw new IndexOutOfRangeException();
                    }
                    return new FontFace(nativeFont.GetFontFace(index));
                }
            }

            /// <summary>
            /// Returns an enumerator that iterates through the collection.
            /// </summary>
            /// <returns>An enumerator that iterates through the collection.</returns>
            public IEnumerator<FontFace> GetEnumerator() {
                for (int i = 0; i < count; ++i) {
                    yield return this[i];
                }
            }

            IEnumerator IEnumerable.GetEnumerator() {
                for (int i = 0; i < count; ++i) {
                    yield return this[i];
                }
            }
        }
    }
}