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

    enum class WheelDeltaMode : std::int32_t
    {
        Pixel = 0,
        Line,
        Page
    };
}
