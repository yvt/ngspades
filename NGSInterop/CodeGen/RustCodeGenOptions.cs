using System;
using System.Collections.Generic;
namespace Ngs.Interop.CodeGen
{
    public sealed class RustCodeGenOptions
    {
        public Dictionary<Type, string> TypeMapping { get; } = new Dictionary<Type, string>
        {
            [typeof(sbyte)] = "i8",
            [typeof(short)] = "i16",
            [typeof(int)] = "i32",
            [typeof(long)] = "i64",
            [typeof(byte)] = "u8",
            [typeof(ushort)] = "u16",
            [typeof(uint)] = "u32",
            [typeof(ulong)] = "u64",
            [typeof(float)] = "f32",
            [typeof(double)] = "f64",
            [typeof(IUnknown)] = "IUnknown",
            [typeof(Guid)] = "::ngscom::IID",
            [typeof(IntPtr)] = "usize",
            [typeof(bool)] = "bool", // not sure :)
        };

        public string NgscomCratePath { get; set; } = "::ngscom";

        public string EnumFlagsDeriveName { get; set; } = "NgsEnumFlags";

        public string EnumFlagsCratePath { get; set; } = "::ngsenumflags";
    }
}
