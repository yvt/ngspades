#pragma once

#include <Utils/Heap.h>
#include <list>
#if defined(_MSC_VER) || __has_include(<optional>)
#include <optional> // C++17
#else
#include <experimental/optional>
#endif

namespace ngs {

class SegmentedHeap
{
public:
    SegmentedHeap(std::size_t segmentSize);
    ~SegmentedHeap();

    class Handle
    {
    public:
        Handle() = default;
        void *Dereference() const { return m_block; }
        void *operator*() const { return Dereference(); }

    private:
        friend class SegmentedHeap;

        Handle(void *block, std::list<Heap>::iterator heapIter)
          : m_block{ block }, m_heapIter{ heapIter }
        {
        }

        void *m_block;
        std::list<Heap>::iterator m_heapIter;
    };

    /**
     * @throws ComException{NS_ERROR_OUT_OF_MEMORY} on memory allocation failure.
     */
    Handle Allocate(std::size_t blockSize);
    void Free(Handle);

private:
    std::size_t const m_segmentSize;
    std::list<Heap> m_heaps;
    std::list<Heap>::iterator m_currentHeapIter;
};
}
