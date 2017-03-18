#include <cassert>
#include <cstdlib>

#include "SegmentedHeap.h"

#include <Utils/ComException.h>

namespace ngs {
SegmentedHeap::SegmentedHeap(std::size_t segmentSize) : m_segmentSize{ segmentSize }
{
}

SegmentedHeap::~SegmentedHeap()
{
}

SegmentedHeap::Handle
SegmentedHeap::Allocate(std::size_t blockSize)
{
    if (blockSize > m_segmentSize / 4) {
        void *block = std::malloc(blockSize);
        if (!block) {
            throw ComException{ NS_ERROR_OUT_OF_MEMORY };
        }
        return { block, m_heaps.end() };
    }

    if (m_heaps.empty()) {
        try {
            m_heaps.emplace_back(m_segmentSize);
        } catch (...) {
            throw ComException{ NS_ERROR_OUT_OF_MEMORY };
        }
        m_currentHeapIter = m_heaps.begin();
    }

    auto firstIter = m_currentHeapIter;
    do {
        void *block = m_currentHeapIter->Allocate(blockSize);
        if (block) {
            return { block, m_currentHeapIter };
        }

        ++m_currentHeapIter;
        if (m_currentHeapIter == m_heaps.end()) {
            m_currentHeapIter = m_heaps.begin();
        }
    } while (m_currentHeapIter != firstIter);

    // All heaps were full - create a new one
    try {
        m_heaps.emplace_back(m_segmentSize);
    } catch (...) {
        throw ComException{ NS_ERROR_OUT_OF_MEMORY };
    }
    m_currentHeapIter = m_heaps.begin();

    void *block = m_currentHeapIter->Allocate(blockSize);
    assert(block);
    return { block, m_currentHeapIter };
}

void
SegmentedHeap::Free(Handle handle)
{
    if (handle.m_heapIter == m_heaps.end()) {
        std::free(handle.m_block);
    } else {
        Heap &heap = *handle.m_heapIter;
        heap.Free(handle.m_block);
        m_currentHeapIter = handle.m_heapIter;
    }
}
}