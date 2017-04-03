using System;
using System.Reflection;
using System.Reflection.Emit;
using System.Linq;
using System.Runtime.InteropServices;
using System.Collections.Generic;

namespace Ngs.Interop.Marshaller
{
    sealed class SimplePointerValueMarshaller : ValueMarshaller
    {
        private sealed class ToNativeGenerator : ValueToNativeMarshallerGenerator
        {
            ILGenerator generator;

            public ToNativeGenerator(ILGenerator generator)
            {
                this.generator = generator;
            }

            public override void EmitToNative(Storage inputStorage, Storage outputStorage)
            {
                inputStorage.EmitLoad();
                outputStorage.EmitStore();
            }
        }

        Type type;

        public SimplePointerValueMarshaller(Type type)
        {
            this.type = type;
        }

        public override ValueToNativeMarshallerGenerator CreateToNativeGenerator(ILGenerator generator)
        {
            return new ToNativeGenerator(generator);
		}

		public override ValueToRuntimeMarshallerGenerator CreateToRuntimeGenerator(ILGenerator generator)
		{
			throw new NotImplementedException();
		}

        public override Type NativeParameterType
        {
            get { return typeof(IntPtr); }
        }
    }
}