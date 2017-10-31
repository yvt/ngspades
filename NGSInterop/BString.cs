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

        /// <summary>
        /// Creates a NgsCOM <c>BString</c> with uninitialized contents.
        /// </summary>
        /// <param name="length">The length of the string measured in bytes.</param>
        /// <returns>A pointer to the newly allocated <c>BString</c>.</returns>
        [System.Security.SecurityCritical]
        public unsafe static IntPtr AllocZeroBString(int length)
        {
            return BStringVTable.Create(length);
        }

        /// <summary>
        /// Creates a NgsCom <c>BString</c> containing the specified string.
        /// </summary>
        /// <param name="str">The contents to initialize the allocated <c>BString</c> with.</param>
        /// <returns>A pointer to the newly allocated <c>BString</c>.</returns>
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

        /// <summary>
        /// Gets a pointer to the string data given the pointer to a <c>BString</c>.
        /// </summary>
        /// <param name="ptr">The pointer to a <c>BString</c>. Must not be a null pointer.</param>
        /// <returns>The pointer to the UTF-8 encoded contents of the string.</returns>
        [System.Security.SecurityCritical]
        public unsafe static byte* GetBStringDataPtr(IntPtr ptr)
        {
            return (byte*)ptr + sizeof(IntPtr) * 2;
        }

        /// <summary>
        /// Gets the length of the string given the pointer to a <c>BString</c>.
        /// </summary>
        /// <param name="ptr">The pointer to a <c>BString</c>. Must not be a null pointer.</param>
        /// <returns>The length of string measured in bytes.</returns>
        [System.Security.SecurityCritical]
        public unsafe static int GetBStringLength(IntPtr ptr)
        {
            return *(int*)((byte*)ptr + sizeof(IntPtr));
        }

        /// <summary>
        /// Gets the contents of the string given the pointer to a <c>BString</c>.
        /// </summary>
        /// <param name="ptr">The pointer to a <c>BString</c>.</param>
        /// <returns>
        /// The contents of the string, or <c>null</c> if <paramref name="ptr" />
        /// was a null pointer.
        /// </returns>
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

        /// <summary>
        /// Deallocates a <c>BString</c>.
        /// </summary>
        /// <remarks>
        /// This method performs no operation if <paramref name="ptr" /> was
        /// a null pointer.
        /// </remarks>
        /// <param name="ptr">The pointer to a <c>BString</c>.</param>
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

    // FIXME: what is the purpose of this struct?
    /// <summary>
    /// Wraps a pointer to a <c>BString</c> and provides an access to various
    /// operations on it.
    /// </summary>
    public struct BStringRef
    {
        /// <summary>
        /// Retrieves a pointer to the <c>BString</c>.
        /// </summary>
        /// <returns>A pointer to the <c>BString</c>.</returns>
        IntPtr Address { get; }

        /// <summary>
        /// Creates a <see cref="BStringRef" /> from a pointer to a <c>BString</c>.
        /// </summary>
        /// <param name="address">The pointer to a <c>BString</c>.</param>
        [System.Security.SecurityCritical]
        BStringRef(IntPtr address)
        {
            this.Address = address;
        }

        /// <summary>
        /// Retrieves whether the pointer is null.
        /// </summary>
        /// <returns><c>true</c> if the pointer is null. <c>false</c> otherwise.</returns>
        public bool IsNull => Address == (IntPtr)0;

        /// <summary>
        /// Retrieves a <see cref="BStringRef" /> with a null pointer.
        /// </summary>
        /// <returns>A <see cref="BStringRef" /> with a null pointer.</returns>
        public static BStringRef Empty => new BStringRef();

        /// <summary>
        /// Creates a <see cref="BStringRef" /> with a pointer to a <c>BString</c>
        /// containing the given string.
        /// </summary>
        /// <param name="str">The string to initialize the <c>BString</c> with.</param>
        /// <returns>
        /// A <see cref="BStringRef" /> pointing the newly created <c>BString</c>.
        /// </returns>
        public static BStringRef Create(string str)
        {
            return new BStringRef(NgscomMarshal.AllocBString(str));
        }

        /// <summary>
        /// Destroys the <c>BString</c>.
        /// </summary>
        /// <remarks>
        /// It will no longer be safe to use the <see cref="BStringRef" /> after
        /// this method was called.
        /// </remarks>
        [System.Security.SecurityCritical]
        public void Free()
        {
            NgscomMarshal.FreeBString(Address);
        }

        /// <summary>
        /// Retrieves the length of the <c>BString</c> measured in bytes.
        /// </summary>
        /// <returns>The length of the <c>BString</c> measured in bytes.</returns>
        public int ByteLength => NgscomMarshal.GetBStringLength(Address);

        /// <summary>
        /// Retrieves the contents of the <c>BString</c>.
        /// </summary>
        /// <returns>The contents of the <c>BString</c>.</returns>
        public override string ToString()
        {
            return NgscomMarshal.BStringToString(Address);
        }
    }
}