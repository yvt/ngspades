#pragma once

#include <NGSCore.h>

namespace ngs {
enum class ColorWriteMask
{
    All = 0xf,
    Red = 1 << 3,
    Green = 1 << 2,
    Blue = 1 << 1,
    Alpha = 1 << 0
};
NGS_DEFINE_FLAGS(ColorWriteMask)
}
