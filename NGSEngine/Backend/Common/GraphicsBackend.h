#pragma once

#include <NGSCore.h>

namespace ngs {

class RenderPass;
class RenderPassDescriptor;

class RenderPipelineState;
class RenderPipelineDescriptor;

class GraphicsBackend : public RefCounted
{
public:
    virtual RenderPass *CreateRenderPass(RenderPassDescriptor *descriptor) = 0;
    virtual RenderPipelineState *CreateRenderPipelineState(RenderPipelineDescriptor *descriptor) = 0;

protected:
    GraphicsBackend();
    virtual ~GraphicsBackend();
};
}
