#pragma once

#include <memory>

#include <Backend/Common/GraphicsBackend.h>

namespace ngs
{
struct MetalGraphicsBackendPrivate;
class MetalGraphicsBackend final : public GraphicsBackend
{
public:

private:
    std::unique_ptr<MetalGraphicsBackendPrivate> m_private;
};
}
