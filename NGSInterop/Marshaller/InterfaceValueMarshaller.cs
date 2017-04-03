using System;
using System.Reflection;
using System.Reflection.Emit;
using System.Linq;
using System.Runtime.InteropServices;
using System.Collections.Generic;

namespace Ngs.Interop.Marshaller
{
	sealed class InterfaceValueMarshaller : ValueMarshaller
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
				// requires CCW
				throw new NotImplementedException();
			}
		}

		private sealed class ToRuntimeGenerator : ValueToRuntimeMarshallerGenerator
		{
			InterfaceValueMarshaller parent;
			ILGenerator generator;

			public ToRuntimeGenerator(ILGenerator generator, InterfaceValueMarshaller parent)
			{
				this.generator = generator;
				this.parent = parent;
			}

			static MethodInfo getRCWForInterfacePtrMethodGeneric = typeof(NgscomMarshal)
				.GetTypeInfo().GetDeclaredMethod(nameof(NgscomMarshal.GetRCWForInterfacePtr));

			public override void EmitToRuntime(Storage inputStorage, Storage outputStorage)
			{
				var getRCWForInterfacePtrMethod = getRCWForInterfacePtrMethodGeneric.MakeGenericMethod(parent.type);

				inputStorage.EmitLoad();
				generator.Emit(OpCodes.Ldc_I4_0); // addRef = false; interface pointer's ownership is transferred to RCW

				generator.EmitCall(OpCodes.Call, getRCWForInterfacePtrMethod, null);

				outputStorage.EmitStore();
			}
		}

		Type type;

		public InterfaceValueMarshaller(Type type)
		{
			if (!type.GetTypeInfo().IsInterface)
			{
				throw new InvalidOperationException();
			}
			this.type = type;
		}

		public override ValueToNativeMarshallerGenerator CreateToNativeGenerator(ILGenerator generator)
		{
			return new ToNativeGenerator(generator);
		}

		public override ValueToRuntimeMarshallerGenerator CreateToRuntimeGenerator(ILGenerator generator)
		{
			return new ToRuntimeGenerator(generator, this);
		}

		public override Type NativeParameterType
		{
			get { return typeof(IntPtr); }
		}
	}
}