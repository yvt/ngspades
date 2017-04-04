using System;
using System.Reflection;
using System.Reflection.Emit;
using System.Linq;
using System.Runtime.InteropServices;
using System.Collections.Generic;

namespace Ngs.Interop.Marshaller
{
	sealed class StringValueMarshaller : ValueMarshaller
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
				// TODO: marshal string values in a correct way
				inputStorage.EmitLoad();
				outputStorage.EmitStore();
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
				// TODO: marshal string values in a correct way
				inputStorage.EmitLoad();
				outputStorage.EmitStore();
			}
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
			get { return typeof(string); } // TODO: this won't work, of course
		}
	}
}