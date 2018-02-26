//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#pragma once

#include "spirvcross.h"

#ifdef __cplusplus
class SpirV2Msl;
#else
typedef struct SpirV2Msl_ SpirV2Msl;
#endif

#ifdef __cplusplus
extern "C" {
#endif


struct SpirV2MslVertexAttr
{
    uint32_t location;
    uint32_t msl_buffer;
    uint32_t msl_offset;
    uint32_t msl_stride;

    SpirVCrossVertexInputRate input_rate;
};

struct SpirV2MslResourceBinding
{
    uint32_t desc_set;
    uint32_t binding;

    uint32_t msl_buffer;
    uint32_t msl_texture;
    uint32_t msl_sampler;

    /// The index of argument buffer. When specified (not `(uint32_t)-1`),
    /// `msl_buffer`, `msl_texture`, and `msl_sampler` point indices into the
    /// argument buffer.
    uint32_t msl_arg_buffer;

    SpirVCrossExecutionModel stage;
};

SpirV2Msl *SpirV2MslCreate(const uint32_t *spirv, uint32_t spirv_count);
void SpirV2MslDestroy(SpirV2Msl *self);
void SpirV2MslAddVertexAttr(SpirV2Msl *self, const SpirV2MslVertexAttr *vertex_attr);
void SpirV2MslAddResourceBinding(SpirV2Msl *self, const SpirV2MslResourceBinding *binding);
SpirVCrossBool SpirV2MslCompile(SpirV2Msl *self);
const char *SpirV2MslGetError(SpirV2Msl *self);
const char *SpirV2MslGetOutputSourceCode(SpirV2Msl *self);

#ifdef __cplusplus
}
#endif
