#pragma once

#include <NGSCore.h>

#include "Texture.h"
#include <Utils/Geometry.h>

namespace ngs {

enum class RenderPassAttachmentLoadAction
{
    DontCare = 0,
    Load,
    Clear
};

enum class RenderPassAttachmentStoreAction
{
    DontCare = 0,
    Store
};

class RenderPassAttachmentDescriptor final : public RefCounted
{
public:
    RenderPassAttachmentDescriptor();

    void SetTexture(Texture *value) { m_texture = value; }
    Texture *GetTexture() { return m_texture; }

    void SetLoadAction(RenderPassAttachmentLoadAction value) { m_loadAction = value; }
    RenderPassAttachmentLoadAction GetLoadAction() { return m_loadAction; }

    void SetStoreAction(RenderPassAttachmentStoreAction value) { m_storeAction = value; }
    RenderPassAttachmentStoreAction GetStoreAction() { return m_storeAction; }

    void SetClearValueFloat(Vector4D value) { m_clearValueFloat = value; }
    Vector4D GetClearValueFloat() { return m_clearValueFloat; }

    void SetClearValueInt(IntVector4D value) { m_clearValueInt = value; }
    IntVector4D GetClearValueInt() { return m_clearValueInt; }

private:
    RefPtr<Texture> m_texture;
    RenderPassAttachmentLoadAction m_loadAction;
    RenderPassAttachmentStoreAction m_storeAction;
    Vector4D m_clearValueFloat;
    IntVector4D m_clearValueInt;
};
}
