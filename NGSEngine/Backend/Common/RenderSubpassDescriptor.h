#pragma once

#include <array>

#include <NGSCore.h>

#include "RenderPassAttachmentReference.h"

namespace ngs {

class RenderSubpassDescriptor final : public RefCounted
{
public:
    RenderSubpassDescriptor();

    void SetColorAttachment(std::size_t index, RenderPassAttachmentReference *value)
    {
        m_colorAttachments.at(index) = value;
    }
    RenderPassAttachmentReference *GetColorAttachment(std::size_t index)
    {
        return m_colorAttachments.at(index);
    }

    void SetDepthAttachment(RenderPassAttachmentReference *value) { m_depthAttachment = value; }
    RenderPassAttachmentReference *GetDepthAttachment() { return m_depthAttachment; }

    void SetStencilAttachment(RenderPassAttachmentReference *value)
    {
        m_stencilAttachment = value;
    }
    RenderPassAttachmentReference *GetStencilAttachment() { return m_stencilAttachment; }

private:
    std::array<RefPtr<RenderPassAttachmentReference>, 8> m_colorAttachments;
    RefPtr<RenderPassAttachmentReference> m_depthAttachment;
    RefPtr<RenderPassAttachmentReference> m_stencilAttachment;
};
}
