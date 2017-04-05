using System;
using System.Reflection;
using System.Reflection.Emit;
using System.Linq;
using System.Runtime.InteropServices;
using System.Collections.Generic;

namespace Ngs.Interop.Marshaller
{
    sealed class SimpleValueMarshaller : ValueMarshaller
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

        	public override void EmitDestructNativeValue(Storage nativeStorage)
			{
			}
        }

		private sealed class ToRuntimeGenerator : ValueToRuntimeMarshallerGenerator
		{
			ILGenerator generator;

			public ToRuntimeGenerator(ILGenerator generator)
			{
				this.generator = generator;
			}

			public override void EmitToRuntime(Storage inputStorage, Storage outputStorage)
			{
				inputStorage.EmitLoad();
				outputStorage.EmitStore();
			}

        	public override void EmitDestructNativeValue(Storage nativeStorage)
			{
			}
		}

        Type type;

        public SimpleValueMarshaller(Type type)
        {
            this.type = type;
		}

		public override ValueToNativeMarshallerGenerator CreateToNativeGenerator(ILGenerator generator)
		{
			return new ToNativeGenerator(generator);
		}

		public override ValueToRuntimeMarshallerGenerator CreateToRuntimeGenerator(ILGenerator generator)
		{
			return new ToRuntimeGenerator(generator);
		}

        public override Type NativeParameterType
        {
            get { return type; }
        }
    }
}