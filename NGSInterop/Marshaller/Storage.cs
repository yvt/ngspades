using System;
using System.Reflection.Emit;
using System.Collections.Generic;

namespace Ngs.Interop.Marshaller
{
	abstract class Storage
	{
		public ILGenerator ILGenerator { get; private set; }
		public Type Type { get; private set; }

		public Storage(ILGenerator generator, Type type)
		{
			this.ILGenerator = generator;
			this.Type = type;
		}

        public abstract void EmitLoad();
        public abstract void EmitLoadAddress();
        public abstract void EmitStore();
    }

    sealed class ParameterStorage : Storage
    {
        int position;

        public ParameterStorage(ILGenerator generator, Type type, int position) :
		base(generator, type)
        {
            this.position = position;
        }

        public override void EmitLoad()
        {
            ILGenerator.Emit(OpCodes.Ldarg, position);
        }

        public override void EmitLoadAddress()
        {
			ILGenerator.Emit(OpCodes.Ldarga, position);
        }
        
        public override void EmitStore()
        {
			ILGenerator.Emit(OpCodes.Starg, position);
        }
    }

    sealed class LocalStorage : Storage
    {
        LocalBuilder local;

		public LocalStorage(ILGenerator generator, LocalBuilder local) :
		base(generator, local.LocalType)
        {
            this.local = local;
        }

        public override void EmitLoad()
        {
			ILGenerator.Emit(OpCodes.Ldloc, local);
        }

        public override void EmitLoadAddress()
        {
			ILGenerator.Emit(OpCodes.Ldloca, local);
        }
        
        public override void EmitStore()
        {
			ILGenerator.Emit(OpCodes.Stloc, local);
        }
    }

	sealed class IndirectStorage : Storage
	{
		Storage baseStorage;
		LocalBuilder temporary;

		static Type Dereference(Type t)
		{
			if (t.IsPointer)
			{
				return t.GetElementType();
			}
			throw new InvalidOperationException($"Type {t.FullName} is not a pointer type.");
		}

		public IndirectStorage(Storage baseStorage)
			: base(baseStorage.ILGenerator, Dereference(baseStorage.Type))
		{
			this.baseStorage = baseStorage;
		}

		public override void EmitLoad()
		{
			baseStorage.EmitLoad();
			ILGenerator.Emit(OpCodes.Ldind_Ref, baseStorage.Type);
		}

		public override void EmitLoadAddress()
		{
			baseStorage.EmitLoad();
		}

		public override void EmitStore()
		{
			if (temporary == null)
			{
				temporary = ILGenerator.DeclareLocal(baseStorage.Type);
			}
			ILGenerator.Emit(OpCodes.Stloc, temporary);
			baseStorage.EmitLoad();
			ILGenerator.Emit(OpCodes.Ldloc, temporary);
			ILGenerator.Emit(OpCodes.Ldind_Ref, baseStorage.Type);
		}
	}
    
}