//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef uint8_t SpirVCrossBool;

#define SpirVCrossBoolTrue  ((SpirVCrossBool)1)
#define SpirVCrossBoolFalse ((SpirVCrossBool)0)

typedef uint8_t SpirVCrossExecutionModel;

enum {
    SpirVCrossExecutionModelVertex = 0,
    SpirVCrossExecutionModelTessellationControl = 1,
    SpirVCrossExecutionModelTessellationEvaluation = 2,
    SpirVCrossExecutionModelGeometry = 3,
    SpirVCrossExecutionModelFragment = 4,
    SpirVCrossExecutionModelGLCompute = 5,
    SpirVCrossExecutionModelKernel = 6,
};

typedef uint8_t SpirVCrossVertexInputRate;

enum {
    SpirVCrossVertexInputRateVertex = 0,
    SpirVCrossVertexInputRateInstance = 1,
};

#ifdef __cplusplus
}
#endif
