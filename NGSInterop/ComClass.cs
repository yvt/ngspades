//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using System.Security;
using System.Threading;

namespace Ngs.Interop
{
    /*
    Imagine this:
    struct ComClassHeader
    {
        public IntPtr vtable;
        public GCHandle<ComClass> handle;
    }
    */

    /// <summary>
    /// A base class of NgsCOM-visible classes, initialized using a runtime
    /// reflection.
    /// </summary>
    /// <remarks>
    /// <para>
    /// This class automatically provides an implementation of
    /// <see cref="IUnknown" /> and exposes all NgsCOM-compatible interfaces
    /// implemented by a derived class.
    /// </para>
    /// <para>
    /// Please consider deriving from <see cref="ComClass&lt;T&gt;" /> instead
    /// of deriving directly from this class. <see cref="ComClass&lt;T&gt;" />
    /// provides a better creation performance.
    /// </para>
    /// </remarks>
    public abstract class ComClass : IUnknown
    {
        IntPtr[] headers;
        GCHandle headersHandle;

        ComClassHeaderInfo[] headerInfos;

        /** COM reference count - defaults to zero. */
        int refCount;

        /// <summary>
        /// Creates an instance of <see cref="ComClass" />.
        /// </summary>
        public ComClass()
        {
            Initialize(new ComClassRuntimeInfo(GetType()));
        }

        internal ComClass(ComClassRuntimeInfo info)
        {
            Initialize(info);
        }

        void Initialize(ComClassRuntimeInfo info)
        {
            headerInfos = info.Headers;
            headers = new IntPtr[headerInfos.Length * 2];
            for (int i = 0; i < headerInfos.Length; ++i)
            {
                var headerInfo = headerInfos[i];
                headers[i * 2] = headerInfo.VTablePtr; // vtable
                headers[i * 2 + 1] = IntPtr.Zero; // handle to this
            }

            headersHandle = GCHandle.Alloc(headers, GCHandleType.Pinned);
        }
        /// <summary>
        /// Releases all the resources used by the <see cref="ComClass" /> class.
        /// </summary>
        ~ComClass()
        {
            headersHandle.Free();
        }

        /// <summary>
        /// Implements <see cref="IUnknown.QueryNativeInterface(ref Guid)" />.
        /// </summary>
        /// <returns>
        /// The pointer to the requested interface.
        /// </returns>
        [SecurityCritical]
        public unsafe IntPtr QueryNativeInterface([In] ref Guid guid)
        {
            var theGuid = guid;
            for (int i = 0; i < headerInfos.Length; ++i)
            {
                var guids = headerInfos[i].Guids;
                if (Array.IndexOf(guids, theGuid) >= 0)
                {
                    AddRef();
                    return IntPtr.Add(headersHandle.AddrOfPinnedObject(), IntPtr.Size * 2 * i);
                }
            }
            const int E_NOINTERFACE = unchecked((int)0x80004002);
            throw new COMException("The specified interface is not available.", E_NOINTERFACE);
        }

        /// <summary>
        /// Implements <see cref="IUnknown.AddRef" />.
        /// </summary>
        /// <returns>The new reference count.</returns>
        [SecurityCritical]
        public uint AddRef()
        {
            int newCount = Interlocked.Increment(ref refCount);
            if (newCount == 1)
            {
                for (int i = 1; i < headers.Length; i += 2)
                {
                    headers[i] = (IntPtr)GCHandle.Alloc(this);
                }
            }
            else if (newCount <= 0)
            {
                // something is wrong...
                throw new InvalidOperationException();
            }
            return (uint)newCount;
        }

        /// <summary>
        /// Implements <see cref="IUnknown.Release" />.
        /// </summary>
        /// <returns>The new reference count.</returns>
        [SecurityCritical]
        public uint Release()
        {
            int newCount = Interlocked.Decrement(ref refCount);
            if (newCount == 0)
            {
                for (int i = 1; i < headers.Length; i += 2)
                {
                    ((GCHandle)headers[i]).Free();
                }
            }
            else if (newCount < 0)
            {
                // something is wrong...
                throw new InvalidOperationException();
            }
            return (uint)newCount;
        }
    }

    /// <summary>
    /// A base class of NgsCOM-visible classes, initialized using a generic
    /// class.
    /// </summary>
    /// <remarks>
    /// <para>
    /// This class automatically provides an implementation of
    /// <see cref="IUnknown" /> and exposes all NgsCOM-compatible interfaces
    /// implemented by a derived class.
    /// </para>
    /// </remarks>
    /// <typeparam name="T">The type of the derived class.</typeparam>
    public abstract class ComClass<T> : ComClass where T : ComClass
    {
        /// <summary>
        /// Creates an instance of <see cref="ComClass&lt;T&gt;" />.
        /// </summary>
        public ComClass() : base(ComClassRuntimeInfo<T>.Instance) { }
    }
}
