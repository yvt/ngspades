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
    
    public abstract class ComClass : IUnknown
    {   
        IntPtr[] headers;
        GCHandle headersHandle;

        ComClassHeaderInfo[] headerInfos;

        /** COM reference count - defaults to zero. */
        int refCount;

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
            for (int i = 0; i < headerInfos.Length; ++i) {
                var headerInfo = headerInfos[i];
                headers[i * 2] = headerInfo.VTablePtr; // vtable
                headers[i * 2 + 1] = IntPtr.Zero; // handle to this
            }

            headersHandle = GCHandle.Alloc(headers, GCHandleType.Pinned);
        }

        ~ComClass()
        {
            headersHandle.Free();
        }

		[SecurityCritical]
		public unsafe IntPtr QueryNativeInterface([In] ref Guid guid)
        {
            var theGuid = guid;
            for (int i = 0; i < headerInfos.Length; ++i)
            {
                var guids = headerInfos[i].Guids;
                if (Array.IndexOf(guids, theGuid) >= 0) {
                    AddRef();
                    return IntPtr.Add(headersHandle.AddrOfPinnedObject(), IntPtr.Size * 2 * i);
                }
            }
			const int E_NOINTERFACE = unchecked((int)0x80004002);
            throw new COMException("The specified interface is not available.", E_NOINTERFACE);
        }

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

    public abstract class ComClass<T> : ComClass where T : ComClass
    {
        public ComClass(): base(ComClassRuntimeInfo<T>.Instance) {}
    }
}
