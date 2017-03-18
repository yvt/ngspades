#pragma once

#include <cstdint>

namespace ngs
{
    enum class FullScreenMode : std::int32_t
    {
        Windowed = 0,
        FullScreenWindow,
        FullScreen
    };
}
