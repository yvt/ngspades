#pragma once

#include <cstdlib>

namespace ngs {

class UninitializedMemoryBlock
{
public:
    /**
     * @throws ComException{ NS_ERROR_OUT_OF_MEMORY }
     */
    UninitializedMemoryBlock(std::size_t size);
    ~UninitializedMemoryBlock() noexcept { std::free(m_data); }

    void *GetData() noexcept { return m_data; }
    const void *GetData() const noexcept { return m_data; }
    std::size_t GetSize() const noexcept { return m_size; }

    // STL compatible interface
    char *begin() noexcept { return m_data; }
    char *end() noexcept { return m_data + m_size; }
    const char *begin() const noexcept { return m_data; }
    const char *end() const noexcept { return m_data + m_size; }

    const char *cbegin() const noexcept { return m_data; }
    const char *cend() const noexcept { return m_data + m_size; }

    std::size_t size() const noexcept { return m_size; }
    constexpr bool empty() const noexcept { return m_size == 0; }

    char *data() noexcept { return m_data; }
    const char *data() const noexcept { return m_data; }

    char &operator[](std::size_t index) noexcept { return m_data[index]; }
    const char &operator[](std::size_t index) const noexcept { return m_data[index]; }

private:
    char *const m_data;
    std::size_t const m_size;
};
}