#include "UninitializedMemoryBlock.h"

#include <Utils/ComException.h>

namespace ngs {

UninitializedMemoryBlock::UninitializedMemoryBlock(std::size_t size)
  : m_data{ reinterpret_cast<char *>(std::malloc(size)) }, m_size(size)
{
    if (!m_data) {
        throw ComException{ NS_ERROR_OUT_OF_MEMORY };
    }
}
}
