#include "RenderPipelineAttachmentDescriptor.h"

namespace ngs {
RenderPipelineAttachmentDescriptor::RenderPipelineAttachmentDescriptor()
  : m_pixelFormat{ PixelFormat::Undefined }
  , m_colorWriteMask{ ColorWriteMask::All }
  , m_enableBlending{ false }
  , m_sourceAlphaBlendFactor{ BlendFactor::One }
  , m_sourceRGBBlendFactor{ BlendFactor::One }
  , m_destinationAlphaBlendFactor{ BlendFactor::One }
  , m_destinationRGBBlendFactor{ BlendFactor::One }
  , m_alphaBlendOperation{ BlendOperation::Add }
  , m_rgbBlendOperation{ BlendOperation::Add }
{
}
}
