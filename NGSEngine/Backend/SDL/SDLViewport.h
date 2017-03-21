#pragma once

#include <IViewport.h>

typedef struct SDL_Window SDL_Window;

namespace ngs {

class SDLViewport : public IViewport
{
public:
    NS_DECL_ISUPPORTS
    NS_DECL_IVIEWPORT

    SDLViewport();

private:
    ~SDLViewport();

    SDL_Window *m_window;
};
}
