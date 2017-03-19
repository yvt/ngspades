#pragma once

#include <NGSCore.h>

namespace ngs {

class RenderPassBase;

class GraphicsBackendBase : public RefCounted
{
public:
    GraphicsBackendBase() {}

protected:
    virtual ~GraphicsBackendBase() {}
};
}
