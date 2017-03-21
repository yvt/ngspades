#pragma once

#include <array>

#include <NGSCore.h>

#include "RenderPassAttachmentDescriptor.h"

namespace ngs {

class RenderPassDescriptor final : public RefCounted
{
public:
    RenderPassDescriptor();

    void SetColorAttachment(std::size_t index, RenderPassAttachmentDescriptor *value)
    {
        m_colorAttachments.at(index) = value;
    }
    RenderPassAttachmentDescriptor *GetColorAttachment(std::size_t index)
    {
        return m_colorAttachments.at(index);
    }

    void SetDepthAttachment(RenderPassAttachmentDescriptor *value) { m_depthAttachment = value; }
    RenderPassAttachmentDescriptor *GetDepthAttachment() { return m_depthAttachment; }

    void SetStencilAttachment(RenderPassAttachmentDescriptor *value)
    {
        m_stencilAttachment = value;
    }
    RenderPassAttachmentDescriptor *GetStencilAttachment() { return m_stencilAttachment; }

private:
    std::array<RefPtr<RenderPassAttachmentDescriptor>, 8> m_colorAttachments;
    RefPtr<RenderPassAttachmentDescriptor> m_depthAttachment;
    RefPtr<RenderPassAttachmentDescriptor> m_stencilAttachment;
};
}
