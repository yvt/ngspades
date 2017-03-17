#pragma once

#include <utility>
#include <vector>
#include <unordered_set>

#include <Utils/UninitializedMemoryBlock.h>

namespace ngs {

class HeapBase
{
public:
    HeapBase(char *storage, std::size_t size) noexcept : m_storage{ storage }, m_size{ size } {}
    ~HeapBase();

    void Initialize() noexcept;
    void *Allocate(std::size_t size) noexcept;
    void Free(void *block) noexcept;

    template <class F>
    void ForEachAllocatedBlock(F callback) noexcept(noexcept(callback()))
    {
        // TODO: call callback(ptr) for each allocated block
    }

private:
    char *m_storage;
    std::size_t m_size;

    // stub implementation
    std::unordered_set<void *> m_allocations;
};

/**
 * Provides a heap memory region and manages memory allocations inside it.
 * Best for allocating many small objects and deallocating all of them quickly.
 *
 * Allocated blocks are 4-byte aligned.
 */
template <class Storage>
class BasicHeap
{
public:
    template <class... Args>
    BasicHeap(Args &&...args) : m_storage{ std::forward<Args>(args)... }
    {
        GetHeapBase().Initialize();
    }

    void *Allocate(std::size_t size)
    {
        return GetHeapBase().Allocate(size);
    }

    void Free(void *block)
    {
        return GetHeapBase().Free(block);
    }

    template <class F>
    void ForEachAllocatedBlock(F callback)
    {
        return GetHeapBase().ForEachAllocatedBlock(callback);
    }

private:
    Storage m_storage;

    HeapBase GetHeapBase()
    {
        return HeapBase{ reinterpret_cast<char *>(m_storage.data()),
                         m_storage.size() * sizeof(m_storage.data()[0]) };
    }
};

using Heap = BasicHeap<UninitializedMemoryBlock>;

}
