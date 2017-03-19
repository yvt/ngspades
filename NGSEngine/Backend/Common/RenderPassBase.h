#pragma once

#include <NGSCore.h>

namespace ngs {

class RenderPassBase : public RefCounted
{
public:
    RenderPassBase() {}

protected:
    virtual ~RenderPassBase() {}
};
}
