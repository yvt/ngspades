#pragma once

#include <IVoxelTerrain.h>
#include <Utils/Geometry.h>

namespace ngs {

class VoxelTerrain final : public IVoxelTerrain
{
public:
    NS_DECL_THREADSAFE_ISUPPORTS
    NS_DECL_IVOXELTERRAIN

    /**
     * Creates a VoxelTerrain instance.
     *
     * @throws ComException
     */
    VoxelTerrain(const IntVector3D &dimensions);

    std::int32_t GetWidth() const { return m_width; }
    std::int32_t GetHeight() const { return m_height; }
    std::int32_t GetDepth() const { return m_depth; }

    TerrainVoxelInfo GetVoxel(IntVector3D voxel);
    // nsresult IVoxelTerrain::SetVoxel(IntVector3 voxel, TerrainVoxelInfo info)

protected:
    ~VoxelTerrain();

private:
    std::int32_t const m_width;
    std::int32_t const m_height;
    std::int32_t const m_depth;
};
}