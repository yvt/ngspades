#include "VoxelTerrain.h"

#include <Utils/ComException.h>

namespace ngs {

NS_IMPL_ISUPPORTS(VoxelTerrain, IVoxelTerrain)

VoxelTerrain::VoxelTerrain(const IntVector3D &dimensions)
  : m_width{ dimensions.x }, m_height{ dimensions.y }, m_depth{ dimensions.z }
{
    if (m_width < 1 || m_height < 1 || m_depth < 1 || m_width > 4096 || m_height > 4096 ||
        m_depth > 128) {
        throw ComException{ NS_ERROR_ILLEGAL_VALUE };
    }
}

VoxelTerrain::~VoxelTerrain()
{
}

TerrainVoxelInfo
VoxelTerrain::GetVoxel(IntVector3D voxel)
{
    NS_NOTYETIMPLEMENTED("GetVoxel");
    return {};
}

/*
 * IVoxelTerrain implementation
 */

NS_IMETHODIMP
VoxelTerrain::GetWidth(std::int32_t *aWidth)
{
    NS_ENSURE_ARG_POINTER(aWidth);
    *aWidth = m_width;
    return NS_OK;
}

NS_IMETHODIMP
VoxelTerrain::GetHeight(std::int32_t *aHeight)
{
    NS_ENSURE_ARG_POINTER(aHeight);
    *aHeight = m_height;
    return NS_OK;
}

NS_IMETHODIMP
VoxelTerrain::GetDepth(std::int32_t *aDepth)
{
    NS_ENSURE_ARG_POINTER(aDepth);
    *aDepth = m_depth;
    return NS_OK;
}

NS_IMETHODIMP
VoxelTerrain::GetVoxel(IntVector3D voxel, TerrainVoxelInfo *_retval)
{
    NS_ENSURE_ARG_POINTER(_retval);
    *_retval = GetVoxel(voxel);
    return NS_OK;
}

NS_IMETHODIMP
VoxelTerrain::SetVoxel(IntVector3D voxel, TerrainVoxelInfo info)
{
    NS_NOTYETIMPLEMENTED("SetVoxel");
    return NS_ERROR_NOT_IMPLEMENTED;
}
}
