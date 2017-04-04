using System;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine {
	[Guid("29e48ae6-bb78-44b3-ba1d-5a3310ff137d")]
	public interface IVoxelTerrain : IUnknown
	{
		int Width { get; }
		int Height { get; }
		int Depth { get; }
		Ngs.Engine.TerrainVoxelInfo GetVoxel(Ngs.Utils.IntVector3 voxel);
		void SetVoxel(Ngs.Utils.IntVector3 voxel, Ngs.Engine.TerrainVoxelInfo info);

	}

}
