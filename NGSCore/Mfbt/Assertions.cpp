#include <cassert>
#include "Assertions.h"

#ifndef DEBUG
MFBT_API MOZ_COLD MOZ_NORETURN MOZ_NEVER_INLINE void
MOZ_CrashOOL(int aLine, const char* aReason)
#else
MFBT_API MOZ_COLD MOZ_NORETURN MOZ_NEVER_INLINE void
MOZ_CrashOOL(const char* aFilename, int aLine, const char* aReason)
#endif
{
    assert(false);
}

#ifndef DEBUG
MFBT_API MOZ_COLD MOZ_NORETURN MOZ_NEVER_INLINE MOZ_FORMAT_PRINTF(2, 3) void
MOZ_CrashPrintf(int aLine, const char* aFormat, ...)
#else
MFBT_API MOZ_COLD MOZ_NORETURN MOZ_NEVER_INLINE MOZ_FORMAT_PRINTF(3, 4) void
MOZ_CrashPrintf(const char* aFilename, int aLine, const char* aFormat, ...)
#endif
{
    assert(false);
}

