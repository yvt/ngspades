using System;
using System.Runtime.InteropServices;
using System.Security;

namespace Ngs.Interop
{
    [Guid("00000000-0000-0000-C000-000000000046")]
    public interface IUnknown
    {
        [SecurityCritical]
        IntPtr QueryNativeInterface([In] ref Guid guid);

        [PreserveSig, SecurityCritical]
        uint AddRef();

        [PreserveSig, SecurityCritical]
        uint Release();
    }
}
