#include "RenderPassAttachmentDescriptor.h"

namespace ngs {
RenderPassAttachmentDescriptor::RenderPassAttachmentDescriptor()
  : m_loadAction{ RenderPassAttachmentLoadAction::DontCare }
  , m_storeAction{ RenderPassAttachmentStoreAction::DontCare }
{
}
}