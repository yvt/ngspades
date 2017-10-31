//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using System.Reflection;
using System.Reflection.Emit;
namespace Ngs.Interop
{
    public static partial class NgscomMarshal
    {

        /*
         * BString in NgsCOM is completely different from XPCOM/MSCOM's BSTR and
         * is defined like this:
         *
         *  class BString {
         *  public:
         *      BString(int32_t length) : m_length{length} {
         *          m_data[length] = 0; // null terminator
         *      }
         *      static BString *Create(int32_t length) {
         *          return new (operator new(sizeof(BString) + length)) BString {length};
         *      }
         *      virtual void Destroy() { // vtable[0]
         *          operator delete(this);
         *      }
         *
		 *      union {
         *      	int32_t const m_length;
		 *			size_t const m_pad;
		 *		};
         *      char m_data[1]; // variable length, UTF-8 encoded
         *  };
         */

        delegate void BStringDestruct(IntPtr ptr);

        [System.Security.SecurityCritical]
        private static class BStringVTable
        {
            public static readonly IntPtr[] vtable = new IntPtr[1];
            static readonly object sync = new object();
            static GCHandle gcHandle;
            static int refCount = 0;

            public static void BStringDestructor(IntPtr ptr)
            {
                Marshal.FreeHGlobal(ptr);
                ReleaseVTable();
            }

            static BStringVTable()
            {
                // FIXME: ahead-of-time generation of this for Mono's Full AOT
                var dynamicMethod = new DynamicMethod("GetBStringDestructorFPtr",
                    typeof(IntPtr), new Type[] { });
                var gen = dynamicMethod.GetILGenerator();
                gen.Emit(OpCodes.Ldftn, typeof(BStringVTable)
                    .GetRuntimeMethod("BStringDestructor", new[] { typeof(IntPtr) }));
                gen.Emit(OpCodes.Ret);
                var dlg = (Func<IntPtr>)dynamicMethod.CreateDelegate(typeof(Func<IntPtr>));
                vtable[0] = dlg();
            }

            static IntPtr GetVTable()
            {
                lock (sync)
                {
                    if (refCount == 0)
                    {
                        gcHandle = GCHandle.Alloc(vtable, GCHandleType.Pinned);
                    }
                    ++refCount;
                    return gcHandle.AddrOfPinnedObject();
                }
            }

            static void ReleaseVTable()
            {
                lock (sync)
                {
                    --refCount;
                    if (refCount == 0)
                    {
                        gcHandle.Free();
                    }
                }
            }

            public unsafe static IntPtr Create(int length)
            {
                if (length < 0)
                {
                    throw new ArgumentOutOfRangeException(nameof(length));
                }
                var ptr = Marshal.AllocHGlobal(length + sizeof(IntPtr) * 2 + 1);
                byte* bptr = (byte*)ptr.ToPointer();
                *((IntPtr*)bptr) = GetVTable();
                *((IntPtr*)(bptr + sizeof(IntPtr))) = IntPtr.Zero;
                *((int*)(bptr + sizeof(IntPtr))) = length;
                bptr[sizeof(IntPtr) * 2 + length] = 0; // null terminator
                return ptr;
            }
        }

        [System.Security.SecurityCritical]
        public unsafe static IntPtr AllocZeroBString(int length)
        {
            return BStringVTable.Create(length);
        }

        [System.Security.SecurityCritical]
        public unsafe static IntPtr AllocBString(string str)
        {
            if (str == null)
            {
                return IntPtr.Zero;
            }
            byte[] bytes = System.Text.Encoding.UTF8.GetBytes(str);
            var bstr = BStringVTable.Create(bytes.Length);
            byte* dataptr = GetBStringDataPtr(bstr);
            Marshal.Copy(bytes, 0, (IntPtr)dataptr, bytes.Length);
            return bstr;
        }

        [System.Security.SecurityCritical]
        public unsafe static byte* GetBStringDataPtr(IntPtr ptr)
        {
            return (byte*)ptr + sizeof(IntPtr) * 2;
        }

        [System.Security.SecurityCritical]
        public unsafe static int GetBStringLength(IntPtr ptr)
        {
            return *(int*)((byte*)ptr + sizeof(IntPtr));
        }

        [System.Security.SecurityCritical]
        public unsafe static string BStringToString(IntPtr ptr)
        {
            if (ptr == (IntPtr)0)
            {
                return null;
            }
            byte[] bytes = new byte[GetBStringLength(ptr)];
            Marshal.Copy((IntPtr)GetBStringDataPtr(ptr), bytes, 0, bytes.Length);
            return System.Text.Encoding.UTF8.GetString(bytes);
        }

        [System.Security.SecurityCritical]
        delegate void DestructIndirect(IntPtr fnptr, IntPtr ptr);

        [System.Security.SecurityCritical]
        public unsafe static void FreeBString(IntPtr ptr)
        {
            var releaseDelegate = DelegateRuntimeInfo<DestructIndirect>.RcfpwDelegate;
            if (ptr == (IntPtr)0)
            {
                return;
            }

            IntPtr* vtable = *((IntPtr**)ptr);

            // call destructor
            IntPtr releaseFunctionPtr = vtable[0];
            if (releaseFunctionPtr == BStringVTable.vtable[0])
            {
                // calli on a function ptr created by GetFunctionPointerForDelegate crashes
                // on .NET Core for some reason
                BStringVTable.BStringDestructor(ptr);
            }
            else
            {
                releaseDelegate(releaseFunctionPtr, ptr);
            }
        }
    }

    public struct BStringRef
    {
        IntPtr Address { get; }

        BStringRef(IntPtr address)
        {
            this.Address = address;
        }

        public bool IsNull => Address == (IntPtr)0;

        public static BStringRef Empty => new BStringRef();

        public static BStringRef Create(string str)
        {
            return new BStringRef(NgscomMarshal.AllocBString(str));
        }

        public void Free()
        {
            NgscomMarshal.FreeBString(Address);
        }

        public int ByteLength => NgscomMarshal.GetBStringLength(Address);

        public override string ToString()
        {
            return NgscomMarshal.BStringToString(Address);
        }
    }
}