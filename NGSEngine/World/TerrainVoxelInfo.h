#pragma once

#include <cstdint>

namespace ngs {
struct TerrainVoxelInfo
{
    std::uint32_t color;
    std::uint16_t kind;
    std::uint8_t health;
};
}
