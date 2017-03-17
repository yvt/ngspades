#include <cstdlib>
#include <cassert>

#include "Heap.h"

namespace ngs {

// TODO: HeapBase: Implement TLSF "a New Dynamic Memory Allocator for Real-Time Systems"

HeapBase::~HeapBase()
{
    for (void *block: m_allocations) {
        std::free(block);
    }
}

void HeapBase::Initialize() noexcept
{
    // TODO: HeapBase
}

void *HeapBase::Allocate(std::size_t size) noexcept
{
    // TODO: HeapBase
    void *block = std::malloc(size);
    assert(block);
    m_allocations.insert(block);
    return block;
}

void HeapBase::Free(void *region) noexcept
{
    // TODO: HeapBase
    std::free(region);
    m_allocations.erase(m_allocations.find(region));
}

}
