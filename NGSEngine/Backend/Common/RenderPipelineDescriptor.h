#pragma once

#include <array>

#include <NGSCore.h>

#include "GPUFunction.h"
#include "RenderPass.h"
#include "RenderPipelineAttachmentDescriptor.h"

namespace ngs {

class RenderPipelineDescriptor final : public RefCounted
{
public:
    RenderPipelineDescriptor();

    void SetRenderPass(RenderPass *value) { m_renderPass = value; }
    RenderPass *GetRenderPass() { return m_renderPass; }

    void SetRenderSubpassIndex(std::size_t value) { m_renderSubpassIndex = value; }
    std::size_t GetRenderSubpassIndex() { return m_renderSubpassIndex; }

    void SetFragmentFunction(GPUFunction *value) { m_fragmentFunction = value; }
    GPUFunction *GetFragmentFunction() { return m_fragmentFunction; }

    void SetVertexFunction(GPUFunction *value) { m_vertexFunction = value; }
    GPUFunction *GetVertexFunction() { return m_vertexFunction; }

    void SetColorAttachment(std::size_t index, RenderPipelineAttachmentDescriptor *value)
    {
        m_colorAttachments.at(index) = value;
    }
    RenderPipelineAttachmentDescriptor *GetColorAttachment(std::size_t index)
    {
        return m_colorAttachments.at(index);
    }

    void SetDepthAttachment(RenderPipelineAttachmentDescriptor *value)
    {
        m_depthAttachment = value;
    }
    RenderPipelineAttachmentDescriptor *GetDepthAttachment() { return m_depthAttachment; }

    void SetStencilAttachment(RenderPipelineAttachmentDescriptor *value)
    {
        m_stencilAttachment = value;
    }
    RenderPipelineAttachmentDescriptor *GetStencilAttachment() { return m_stencilAttachment; }

private:
    RefPtr<RenderPass> m_renderPass;
    std::size_t m_renderSubpassIndex;
    RefPtr<GPUFunction> m_fragmentFunction;
    RefPtr<GPUFunction> m_vertexFunction;
    std::array<RefPtr<RenderPipelineAttachmentDescriptor>, 8> m_colorAttachments;
    RefPtr<RenderPipelineAttachmentDescriptor> m_depthAttachment;
    RefPtr<RenderPipelineAttachmentDescriptor> m_stencilAttachment;
};
}
