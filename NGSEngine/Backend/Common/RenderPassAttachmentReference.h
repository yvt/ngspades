#pragma once

#include <NGSCore.h>

namespace ngs {
class RenderPassAttachmentReference final : public RefCounted
{
public:
    RenderPassAttachmentReference();

    void SetAttachmentIndex(std::size_t value) { m_attachmentIndex = value; }
    std::size_t GetAttachmentIndex() { return m_attachmentIndex; }

private:
    std::size_t m_attachmentIndex;
};
}
