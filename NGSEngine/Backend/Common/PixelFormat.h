#pragma once

namespace ngs
{
enum class PixelFormat
{
    Undefined = 0,
    RGBA8Unorm,
    // TODO: more pixel formats...
    //       (intersection of Vulkan (VkFormat) and Metal (MTLPixelFormat))
};
}
