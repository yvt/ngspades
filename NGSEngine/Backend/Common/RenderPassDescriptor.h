#pragma once

#include <vector>

#include <NGSCore.h>

#include "RenderPassAttachmentDescriptor.h"
#include "RenderSubpassDescriptor.h"

namespace ngs {

class RenderPassDescriptor final : public RefCounted
{
public:
    RenderPassDescriptor();

    void SetAttachment(std::size_t index, RenderPassAttachmentDescriptor *value)
    {
        if (index + 1 > m_attachments.size()) {
            m_attachments.resize(index + 1);
        }
        m_attachments.at(index) = value;
    }
    RenderPassAttachmentDescriptor *GetAttachment(std::size_t index)
    {
        if (index >= m_attachments.size()) {
            return nullptr;
        }
        return m_attachments.at(index);
    }
    std::size_t GetAttachmentCount() { return m_attachments.size(); }

    void SetSubpass(std::size_t index, RenderSubpassDescriptor *value)
    {
        if (index + 1 > m_subpasses.size()) {
            m_subpasses.resize(index + 1);
        }
        m_subpasses.at(index) = value;
    }
    RenderSubpassDescriptor *GetSubpass(std::size_t index)
    {
        if (index >= m_subpasses.size()) {
            return nullptr;
        }
        return m_subpasses.at(index);
    }
    std::size_t GetSubpassCount() { return m_subpasses.size(); }

private:
    std::vector<RefPtr<RenderPassAttachmentDescriptor>> m_attachments;
    std::vector<RefPtr<RenderSubpassDescriptor>> m_subpasses;
};
}
