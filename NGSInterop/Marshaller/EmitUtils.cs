using System;
using System.Reflection;
using System.Reflection.Emit;
using System.Linq;
namespace Ngs.Interop.Marshaller
{
    static class EmitExtensions
    {
        static readonly MethodInfo intPtrExplicitPointerConv = typeof(IntPtr).GetRuntimeMethods()
            .First((m) => m.Name == "op_Explicit" && m.ReturnType == typeof(void*));

        public static void EmitIntPtrToPointer(this ILGenerator generator)
        {
            generator.Emit(OpCodes.Call, intPtrExplicitPointerConv);
        }
    }
}
