//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#include <memory>
#include <new>
#include <stdexcept>

#include "../libspirvcross/spirv_msl.hpp"
#include "spirv2msl.h"

using spirv_cross::MSLVertexAttr;
using spirv_cross::MSLResourceBinding;
using spirv_cross::MSLIndirectArgument;
using spirv_cross::CompilerGLSL;
using spirv_cross::CompilerMSL;

class SpirV2Msl
{
public:
    SpirV2Msl(const uint32_t *spirv, uint32_t spirv_count) noexcept
    {
        try {
            compiler.reset(new CompilerMSL{ spirv, static_cast<size_t>(spirv_count) });

            CompilerGLSL::Options options = compiler->get_common_options();
            options.vertex.flip_vert_y = true;
            compiler->set_common_options(options);

            CompilerMSL::Options msl_options = compiler->get_msl_options();
            msl_options.set_msl_version(2, 0, 0);
            compiler->set_msl_options(msl_options);
        } catch (const std::exception &ex) {
            last_error = ex.what();
        }
    }

    void AddVertexAttr(const SpirV2MslVertexAttr *vertex_attr) noexcept
    {
        if (!compiler) {
            return;
        }
        try {
            MSLVertexAttr va;
            va.location = vertex_attr->location;
            va.msl_buffer = vertex_attr->msl_buffer;
            va.msl_offset = vertex_attr->msl_offset;
            va.msl_stride = vertex_attr->msl_stride;
            switch (vertex_attr->input_rate) {
                case SpirVCrossVertexInputRateVertex:
                    va.per_instance = false;
                    break;
                case SpirVCrossVertexInputRateInstance:
                    va.per_instance = true;
                    break;
                default:
                    throw std::logic_error("invalid input_rate");
            }
            vertex_attrs.push_back(va);
        } catch (const std::exception &ex) {
            last_error = ex.what();
            compiler.reset();
        }
    }

    void AddResourceBinding(const SpirV2MslResourceBinding *binding)
    {
        if (!compiler) {
            return;
        }
        try {
            MSLResourceBinding rb;
            rb.desc_set = binding->desc_set;
            rb.binding = binding->binding;
            rb.msl_buffer = binding->msl_buffer;
            rb.msl_texture = binding->msl_texture;
            rb.msl_sampler = binding->msl_sampler;
            rb.msl_argument_buffer = binding->msl_arg_buffer;
            switch (binding->stage) {
                case SpirVCrossExecutionModelVertex:
                    rb.stage = spv::ExecutionModelVertex;
                    break;
                case SpirVCrossExecutionModelTessellationControl:
                    rb.stage = spv::ExecutionModelTessellationControl;
                    break;
                case SpirVCrossExecutionModelTessellationEvaluation:
                    rb.stage = spv::ExecutionModelTessellationEvaluation;
                    break;
                case SpirVCrossExecutionModelGeometry:
                    rb.stage = spv::ExecutionModelGeometry;
                    break;
                case SpirVCrossExecutionModelFragment:
                    rb.stage = spv::ExecutionModelFragment;
                    break;
                case SpirVCrossExecutionModelGLCompute:
                    rb.stage = spv::ExecutionModelGLCompute;
                    break;
                case SpirVCrossExecutionModelKernel:
                    rb.stage = spv::ExecutionModelKernel;
                    break;
                default:
                    throw std::logic_error("invalid stage");
            }
            rb.is_depth_texture = binding->is_depth_texture;
            bindings.push_back(rb);
        } catch (const std::exception &ex) {
            last_error = ex.what();
            compiler.reset();
        }
    }

    void AddIndirectArgument(const SpirV2MslIndirectArgument *argument)
    {
        if (!compiler) {
            return;
        }
        try {
            MSLIndirectArgument ia;
            ia.msl_argument_buffer = argument->msl_arg_buffer;
            ia.msl_argument = argument->msl_arg;
            ia.msl_type = argument->msl_type;
            indirect_arguments.push_back(ia);
        } catch (const std::exception &ex) {
            last_error = ex.what();
            compiler.reset();
        }
    }

    SpirVCrossBool Compile()
    {
        if (!compiler) {
            return SpirVCrossBoolFalse;
        }
        try {
            output_msl = compiler->compile(&vertex_attrs, &bindings, &indirect_arguments);
            return SpirVCrossBoolTrue;
        } catch (const std::exception &ex) {
            last_error = ex.what();
            return SpirVCrossBoolFalse;
        }
    }

    const char *GetError() { return last_error.c_str(); }

    const char *GetOutputSourceCode() { return output_msl.c_str(); }

private:
    // can be null if any error occurs
    std::unique_ptr<CompilerMSL> compiler;

    std::string last_error;
    std::string output_msl;

    std::vector<MSLVertexAttr> vertex_attrs;
    std::vector<MSLResourceBinding> bindings;
    std::vector<MSLIndirectArgument> indirect_arguments;
};

SpirV2Msl *
SpirV2MslCreate(const uint32_t *spirv, uint32_t spirv_count)
{
    return new (std::nothrow) SpirV2Msl(spirv, spirv_count);
}

void
SpirV2MslDestroy(SpirV2Msl *self)
{
    delete self;
}

void
SpirV2MslAddVertexAttr(SpirV2Msl *self, const SpirV2MslVertexAttr *vertex_attr)
{
    self->AddVertexAttr(vertex_attr);
}

void
SpirV2MslAddResourceBinding(SpirV2Msl *self, const SpirV2MslResourceBinding *binding)
{
    self->AddResourceBinding(binding);
}

void
SpirV2MslAddIndirectArgument(SpirV2Msl *self, const SpirV2MslIndirectArgument *ia)
{
    self->AddIndirectArgument(ia);
}

SpirVCrossBool
SpirV2MslCompile(SpirV2Msl *self)
{
    return self->Compile();
}

const char *
SpirV2MslGetError(SpirV2Msl *self)
{
    return self->GetError();
}

const char *
SpirV2MslGetOutputSourceCode(SpirV2Msl *self)
{
    return self->GetOutputSourceCode();
}
