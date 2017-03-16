#include <cstdlib>
#include <string>

#include "ComException.h"

namespace ngs {

namespace {
    std::string GetErrorMessageForNSResult(nsresult result)
    {
        char buf[128];
        buf[127] = 0;
        snprintf(buf, 127, "COM HRESULT: 0x%08x", (int)result);
        return buf;
    }
}

ComException::ComException(nsresult result)
  : std::runtime_error{ GetErrorMessageForNSResult(result) }, m_nsResult{ result }
{
}

ComException::ComException(nsresult result, const char *message)
  : std::runtime_error{ message }, m_nsResult{ result }
{
}

ComException::ComException(nsresult result, const std::string &message)
  : std::runtime_error{ message }, m_nsResult{ result }
{
}
}