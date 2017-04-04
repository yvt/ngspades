using System;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine {
	[Guid("d0c02ad7-4185-403d-b5c3-79ecfcc57c1f")]
	public interface IEngine : IUnknown
	{
		IVoxelTerrain CreateVoxelTerrain(Ngs.Utils.IntVector3 dimensions);
	}

}
