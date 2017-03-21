#pragma once

#include <NGSCore.h>

#include "BlendFactor.h"
#include "BlendOperation.h"
#include "ColorWriteMask.h"
#include "PixelFormat.h"
#include "Texture.h"

namespace ngs {

class RenderPipelineAttachmentDescriptor final : public RefCounted
{
public:
    RenderPipelineAttachmentDescriptor();

    void SetPixelFormat(PixelFormat value) { m_pixelFormat = value; }
    PixelFormat GetPixelFormat() { return m_pixelFormat; }

    /** Only valid for color attachments. (Ignored for other attachments) */
    void SetEnableBlending(bool value) { m_enableBlending = value; }
    bool GetEnableBlending() { return m_enableBlending; }

    /** Only valid for color attachments. (Ignored for other attachments) */
    void SetColorWriteMask(ColorWriteMask value) { m_colorWriteMask = value; }
    ColorWriteMask GetColorWriteMask() { return m_colorWriteMask; }

    /** Only valid for color attachments. (Ignored for other attachments) */
    void SetSourceAlphaBlendFactor(BlendFactor value) { m_sourceAlphaBlendFactor = value; }
    BlendFactor GetSourceAlphaBlendFactor() { return m_sourceAlphaBlendFactor; }

    /** Only valid for color attachments. (Ignored for other attachments) */
    void SetSourceRGBBlendFactor(BlendFactor value) { m_sourceRGBBlendFactor = value; }
    BlendFactor GetSourceRGBBlendFactor() { return m_sourceRGBBlendFactor; }

    /** Only valid for color attachments. (Ignored for other attachments) */
    void SetDestinationAlphaBlendFactor(BlendFactor value)
    {
        m_destinationAlphaBlendFactor = value;
    }
    BlendFactor GetDestinationAlphaBlendFactor() { return m_destinationAlphaBlendFactor; }

    /** Only valid for color attachments. (Ignored for other attachments) */
    void SetDestinationRGBBlendFactor(BlendFactor value) { m_destinationRGBBlendFactor = value; }
    BlendFactor GetDestinationRGBBlendFactor() { return m_destinationRGBBlendFactor; }

    /** Only valid for color attachments. (Ignored for other attachments) */
    void SetAlphaBlendOperation(BlendOperation value) { m_alphaBlendOperation = value; }
    BlendOperation GetAlphaBlendOperation() { return m_alphaBlendOperation; }

    /** Only valid for color attachments. (Ignored for other attachments) */
    void SetRGBBlendOperation(BlendOperation value) { m_rgbBlendOperation = value; }
    BlendOperation GetRGBBlendOperation() { return m_rgbBlendOperation; }

private:
    PixelFormat m_pixelFormat;
    ColorWriteMask m_colorWriteMask;
    bool m_enableBlending;
    BlendFactor m_sourceAlphaBlendFactor;
    BlendFactor m_sourceRGBBlendFactor;
    BlendFactor m_destinationAlphaBlendFactor;
    BlendFactor m_destinationRGBBlendFactor;
    BlendOperation m_alphaBlendOperation;
    BlendOperation m_rgbBlendOperation;
};
}
