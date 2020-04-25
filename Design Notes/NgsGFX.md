
Nightingales GFX
================

- SPIRV processing
    - https://github.com/google/rspirv
- Vulkan
    - http://www.adriancourreges.com/blog/2016/09/09/doom-2016-graphics-study/
    - http://gpuopen.com/vulkan-barriers-explained/
    - https://www.reddit.com/r/vulkan/comments/47tc3s/differences_between_vkfence_vkevent_and/
    - http://boxbase.org/entries/2016/mar/7/vulkan-api-overview-3/
    - Debug markers: https://vulkan.lunarg.com/doc/view/1.0.30.0/linux/vkspec.chunked/ch32s01.html
    - [VK_KHR_surface](http://vulkan-spec-chunked.ahcox.com/apes02.html)
- Hardware Database
    - http://vulkan.gpuinfo.org
- API design
    - https://github.com/brson/rust-api-guidelines/blob/master/README.md
- Metal
    - https://developer.apple.com/library/content/documentation/Miscellaneous/Conceptual/MetalProgrammingGuide/ResourceHeaps/ResourceHeaps.html
    - https://developer.apple.com/reference/metal
- D3D12
    - [`D3D12_RESOURCE_STATES`](https://msdn.microsoft.com/en-us/library/windows/desktop/dn986744(v=vs.85).aspx)
- WebGPU
    - [Command Queue investigations](https://github.com/gpuweb/gpuweb/issues/22)

Observations
------------

### Intra-pass barrier

Vulkan:

- (Render subpass) The synchronization scope of `vkCmdPipelineBarrier` inside a render pass is limited to the subpass.

Metal:

- (Render encoder) `textureBarrier` `Ensures that any texture reads issued after the barrier can safely read from any rendering to those textures performed before the barrier.` I guess its intended usage is for post-processing without ping-pong buffers

Conclusion:

- Not implemented due to inconcise definition of `textureBarrier`.

### Encoder-Encoder barrier
    
Vulkan: 

- *Submission order* is established between command buffers submitted to the same queue. The first and second synchronization scope of `vkCmdPipelineBarrier` (which inserts a memory/execution barrier) includes *all* commands submitted to the same queue, before or after the submission of this command as per the *submission order*. 
- A render pass inserts memory barriers automatically using subpass dependencies provided by the user. There are external dependencies whose synchronization scope includes all commands outside the render pass.
- `VkEvent` provides intra-queue synchronization.
- No implicit ordering between `vkCmdResetEvent` and `vkCmdWaitEvents` submitted after it?

Metal without automatic hazard tracking:

- `updateFence` and `useFence` is used to establish the ordering between one encoder (producer) and multiple encoders (consumers).
- In Vulkan's terms, `updateFence`'s first synchronization scope does not include commands from command encoder preceding the current one. This is shown in the [Listing 13-3](https://developer.apple.com/library/content/documentation/Miscellaneous/Conceptual/MetalProgrammingGuide/ResourceHeaps/ResourceHeaps.html#//apple_ref/doc/uid/TP40014221-CH16-DontLinkElementID_20).
- One `MTLFence` is required for every overlapping wait request.

Observation:

- Implementing `vkCmdPipelineBarrier` using `MTLFence` is infeasible since producer/consumer relationship must be established for every combination of preceding encoders and subsequent encoders. The other way around is quite easy, but is a little bit over-conservative.
- `vkCmdPipelineBarrier` also provides image layout transitions and queue family ownership transfers, so `CommandEncoder::barrier` is still required
- `vkCmdPipelineBarrier` provides memory barriers with a fine granularity but drivers actually [don't respsect them](https://github.com/mesa3d/mesa/blob/master/src/amd/vulkan/radv_cmd_buffer.c#L3228). Maybe a global memory barrier suffices.

Conclusion:

- Provide a new synchronization primitive that provides a similar functionality to `MTLFence` but allow more fine specification of access types and pipeline stages. **ISSUE** when are these specified? 
- Only can be used to define a producer/consumer relationship within the same command buffer. **ISSUE** alleviate this restriction?
- For subpass dependencies, memory barriers are inserted automatically. For external dependencies, the user must include them in the subpass dependency specification and **at the same time** must use this new synchronization primitive./
- Vulkan: realized with a pipeline barrier with a global memory barrier. **ISSUE** Maybe use `VkEvent`?
- Metal: directly translated to `MTLFence`. Internal subpass dependencies are converted to `MTLFence` implicitly.
- Metal: Specify `MTLResourceHazardTrackingModeUntracked` to all resources by default (after Metal 2 became available publicly).
- This primitive is named `Fence`. The existing `Fence` (which provides a `VkFence` functionality) will be renamed to `Event`.
- Explicit layout transitions and queue ownership transfers are still required.

### CB-CB barrier

Vulkan:

- `vkCmdPipelineBarrier`'s synchronization scope extends to entire the ordered list of commands sent to the same queue.

Metal:

- CB-CB ordering are [guaranteed](https://developer.apple.com/library/content/documentation/Miscellaneous/Conceptual/MetalProgrammingGuide/Cmd-Submiss/Cmd-Submiss.html#//apple_ref/doc/uid/TP40014221-CH3-SW14). `All command buffers sent to a single queue are guaranteed to execute in the order in which the command buffers were enqueued.`
- Does this imply a memory/execution barrier on a CB boundary? Does this mean `MTLFence` is unnecessary for CB-CB barrier?
- This does not seem to apply the case when automatic hazard tracking is disabled. In this case, the user must use `addScheduledHandler` to manually schedule command buffers. See the [Listing 13-4](https://developer.apple.com/library/content/documentation/Miscellaneous/Conceptual/MetalProgrammingGuide/ResourceHeaps/ResourceHeaps.html#//apple_ref/doc/uid/TP40014221-CH16-DontLinkElementID_20). It also says that this method cannot be used for inter-queue sequencing. That implies waiting for a command buffer completion should be enough to establish a full memory barrier, because otherwise there would be no way to move data between multiple queues.

Conclusion:

- Vulkan: Maybe distribute encoders among hardware queues and establish ordering with `vkSemaphore` (inter-queue sync) and `vkEvent` (intra-queue sync)?
- Metal: commit buffers one by one via `addScheduledHandler` so ordering is done as intended

### Hardware Queues

Vulkan:

- Every IHV exposes a different set of queue families.
    - Intel only has only one universal queue. http://vulkan.gpuinfo.org/displayreport.php?id=1555#queuefamilies
    - AMD has one universal, multiple compute, and multiple transfer queues. http://vulkan.gpuinfo.org/displayreport.php?id=1541#queuefamilies
        - Cannot utilize its Async Compute feature without using multiple queue families, presumably
    - NVIDIA has multiple universal and one transfer queue. http://vulkan.gpuinfo.org/displayreport.php?id=1553#queuefamilies
- How sync primitives are actually implemented?
    - Intel
        - Semaphores are no-op (in most cases) because there is only one software/hardware queue per device
    - AMD
        - Events are implemented by polling
        - Semaphores are implemented using a kind of dependency tracking provided by the hardware?
        - According to *Vulkan Fast Paths* by AMD, events are actually usable for GPU-GPU sync for compute tasks
    - NVIDIA
        - No information available
- To provide a functionality similar to Metal, a command scheduler has to be implemented. Is it possible to incorporate `Fence`? 
    - The goal of this algorithm:
        - Keeping all queues as busy as possible i.e. stall time in each queue has to be minimized.
    - The algorithm assumes the following:
        - for every `Fence` update, only one type of encoder waits to that fence.
    - Signature:
        - `SP_n` - semaphore pool for the queue `n`
        - `S(f)` - semaphore set assobiated with the fence `f`
    - The first step of the algorithm is choosing a queue for each type of encoders: graphics, compute, and blit. 
    - When command buffers were submitted to a NgsGFX queue, the command scheduler takes place, goes through the command pass list `C` (created by concatenating all passes from all command buffers) and constructs command batches `T_n` (each of which is a list of command buffers to be submitted) for each queue.
    - For each command pass `c_n`, we examie the set of fences waited for here. For each fence `f`:
        - If the state of fence is **Initial**, do nothing.
        - If the state of fence is **Signalled**, take one element `s` from `S(f)`.
            - If there is `s`, add a **wait** operation for `s` to the command pass.
            - If there is no `s`, we need to takae a slow path: 
    - ...wait, if we want to use multiple queue families, that means disabling AMD GCN's DCC on images -- Let's use the universal queue for now...
    - **UPDATE**: (2017-06-29) a concept named *device engine* was introduced. Resource ownership transfer between engines is explicit, so this option is now feasible.

Metal:

- Does not expose hardware queues. Arbitrarily created queues are all universal ones.
- If they are all mapped to the universal queue, that means the benefit of GCN's asynchoronous compute queues are completely wasted. I imagine encoders are mapped to multiple hardware queues individually.  

Conclusion:

- Implement a Metal-style fence.
- Vulkan: use the universal queue (for now) and implement a proper scheduler eventually
- Introduce a concept named *device engine* that allows the users to specify which hardware queue the commands should be sent to.

### Host-Device memory barrier

Vulkan:

- Fences do *not* ensure a device-host memory barrier. An explicit call to `vkCmdPipelineBarrier` is required.
    - 6.3. "Fences": `Signaling a fence and waiting on the host does not guarantee that the results of memory accesses will be visible to the host`
- Command submission ensures a host-device memory barrier.
    - 6.1.3. "Access Types": `The vkQueueSubmit command automatically guarantees that host writes flushed to VK_ACCESS_HOST_WRITE_BIT are made available if they were flushed before the command executed`
    - 6.9. "Host Write Ordering Guarantees"
        - `The first access scope includes all host writes to mappable device memory that are either coherent, or have been flushed with vkFlushMappedMemoryRanges.`
        - `The second access scope includes all memory access performed by the device.`

Metal:

- Seems automatic? I am feeling like Metal's documentation is way too much relying on the developers' intuition.

Conclusion:

- Add a command to establish a device-host/host-device memory barrier.

### Presentation / swap chains

Vulkan:

(provide some skeletons)

Metal:

(provide some skeletons)

### Descriptor set

### Heap

Vulkan:

- The developer must allocate a large chunk using one of *memory types* and suballocate from it manually
- No native support for non-heap resources (the number of allocations is quite limited)
- Buffers and images only can be created on the specific memory types depending on their properties
    - Indicated by `VkMemoryRequirements::memoryTypeBits`
    - 11.6. "Resource Memory Association": the set of properties that can affect `memoryTypeBits` is quite restricted
        - For buffers, `memoryTypeBits` is a function of `VkBufferCreateInfo::{flags, usage}`. Further restrictions apply: for the same value of `flags`, the subset relation on `memoryTypeBits` is monotonic regarding the superset relation of `usage`. (In other words, the less `usage` the more `memoryTypeBits` you get, but you'll never lose any bits)
        - For color images, `memoryTypeBits` is a function of `VkImageCreateInfo::{tiling, flags, usage}`. For `flags`, only `VK_IMAGE_CREATE_SPARSE_BINDING_BIT` is regarded. For `usage`, only `VK_IMAGE_USAGE_TRANSIENT_ATTACHMENT_BIT` is regarded.
        - For depth stencil images, `memoryTypeBits` is a function of `VkImageCreateInfo::{tiling, flags, usage, format}`. Ditto.

Metal:

- Native support: `MTLHeap` (iOS 11 and later, macOS 10.13 and later)
- Aliasing is supported, but the guide says it is not allowed for some render target types: https://developer.apple.com/library/content/documentation/Miscellaneous/Conceptual/MetalProgrammingGuide/ResourceHeaps/ResourceHeaps.html#//apple_ref/doc/uid/TP40014221-CH16-DontLinkElementID_22

Conclusion:

- Let the developer specify which kinds of resources are going to be allocated from the created heap in `HeapDescription`.
    - For buffers, `flags` and `usage` should be sufficient
    - For images: disallow transient attachments. Require the value of `format`.
- Add the global or universal heap.
    - Metal: has a native support for non-heap resources
    - Vulkan: Implement a custom allocator. Large resources are allocated from device memory directly while small ones are suballocated from large chunks.
    - Issue: just one per device or arbitrary number? --> Let users create an arbitrary number of heaps
        - There is an advantage to allow an arbitrary number of universal heaps: gives users an opportunity to improve the efficiency of multi-thread operations

### Depth/Stencil Texture

Vulkan:

- To sample from depth/stencil textures, `VkImageView`'s image aspect flags must be used to specify which one to read, depth or stencil.

Metal:

- Needs a special shader type named `depth2d` to read depth textures.
- SPIRV-Cross reads `OpTypeImage`'s `Depth` field to decide the texture type. However, (maybe) there is no known compiler that actually emits values other than "not a depth image" (0), except for "shadow samplers". 
    - Modifying the existing glslang compiler does not seem to easy

Conclusion:

- Add "depth image" descriptor type? And then inject that info into SPIRV-Cross.

## TODOs

- Metal 2
    - Add `use_resource`, `use_heap` so we can support Metal 2 easily?
        - The reason we need this is that `useResource` has to be called before adding draw commands using the resources in the same encoder.
        - [useResource](https://developer.apple.com/documentation/metal/mtlrendercommandencoder/2866168-useresource): `Call this method before issuing any draw calls that may access the resource. `
        - `useResource` does not have to be called for color attachments
        - Vulkan does not need this at all (at least for non-sparse resources) because it considers all resources resident all the times.
- Metal
    - metal-rs: shared memory support. `OpVariable`s with the storage class `Workgrouo` need to be supported. Most variables now work well, but some types including `float4x4` need special treatments because they do not have default constructors for the `threadgroup` address space.
    - Is the maximum invocation count dependent on the shader? On Iris Graphics 550 and some null shader it was 256, but will it change for other shaders?
    - Depth/stencil images need special shader types
    - Descriptor binding check is over-conservative - some bindings may not be "statically used".
    - `-[_MTLCommandEncoder dealloc]:72: failed assertion 'Command encoder released without endEncoding'`
    - Optimize descriptor set allocation 
- Vulkan
    - more efficient descriptor set allocation
    - device lost might result in an application hang because `vk::Fence` is never signalled. (Try running `comp1` with an extraordinary workgroup count)
    - cannot recover from a device lost because swapchain cannot be freed without a successful `LlFence::check_fence` (Try running `ngspf`'s `basic` and then disabling the display adapter)
    - reuse temporarily created `vk::Fence`s
    - Optimize descriptor sets lifetime management especially for the case where deallocation is disabled by `DescriptorPoolDescription`
- `ImageView` needs an image aspect to decide which one to read from depth + stencil combined images
    - Note: in Vulkan, image aspect will not limit access their usage as depth/stencil attachment
- API naming and design
    - Add "depth image" descriptor type
    - `DrawableInfo::extents` `ImageDescription::extent`
    - `DescriptorType::ConstantBuffer` should be renamed to `UniformBuffer`
    - Descriptor set should support updates of `CombinedImageAndSampler` with immutable samplers
    - Uniform/constant buffer offsets have alignment requirements
    - `BitFlags<ColorWriteMask>` needs a type alias
    - Maybe `FramebufferDescription`'s extents would better be specified with `Vector3<u32>` for consistency
    - Linear images needs a function to query their layouts
    - Memoryless resources: remove? Or at least we should indicate its support with `DeviceLimits`
    - Layered rendering? Maybe we don't need it or support it, but at least we should make the API compatible with it
    - Consider requiring layout transitions for buffers (to make it D3D12-proof)
        - Question: do users have to specify the current layout on every API that requires buffers?
    - Come up with a better name for `ImageFlag`
    - Depth bias' clamp value is an optional feature!
    - `SamplerAddressMode::MirroredClampToEdge` is an optional feature!
    - Specify the index buffer type in `GraphicsPipelineDescription` for D3D12 compatibility
- Examples
    - Add test with a compute shader that performs matrix multiplication
    - `CAMetalLayer` might return a null drawable if a frame drop occurs on the window server. We need to wait for a while if we get `NotReady`.
- Add query objects
- support Wayland

Not TODO
--------

- Q-Q synchronnization
    - Not supported because of insufficient support by Metal.
- `memoryBarrier()`, `OpMemoryBarrier`
    - Lack of support by Metal SL.

Vulkan vs Metal
---------------

### Pipeline States Checklist (Jul 11, 2017)

Compiled pipeline state objects:

- Vulkan: `VkPipeline` (P)
- Metal: `MTLRenderPipelineState` (RPS), `MTLDepthStencilState` (DSS)
- NgsGFX: `GraphicsPipeline` (GP), `StencilState` (SS). GP may have a static SS.

|         State          |                  Vulkan                  |  Metal  |                 NgsGFX API                |
| ---------------------- | ---------------------------------------- | ------- | ----------------------------------------- |
| Viewport               | **static**/dynamic                       | dynamic | GP/dynamic                                |
| Scissor rect           | **static**/dynamic                       | dynamic | GP/dynamic                                |
| Cull mode              | **static**                               | dynamic | GP                                        |
| Front face             | **static**                               | dynamic | GP                                        |
| Depth clip mode        | **static** (needs `depthClampEnable`)    | dynamic | GP + limits. **needs validation**         |
| Triangle fill mode     | **static** (needs `fillModeNonSolid`)    | dynamic | GP + limits. **needs validation**         |
| Line width             | **static**/dynamic                       | N/A     | N/A                                       |
| Depth bias             | **static**/dynamic                       | dynamic | GP/dynamic                                |
| Blend constants        | **static**/dynamic                       | dynamic | GP/dynamic                                |
| Depth bounds           | **static**/dynamic (needs `depthBounds`) | N/A     | GP/dynamic + limits. **needs validation** |
| Stencil operations     | **static**                               | **DSS** | GP                                        |
| Stencil masks          | **static**/dynamic                       | **DSS** | GP/SS                                     |
| Stencil reference      | **static**/dynamic                       | dynamic | GP/dynamic                                |
| Depth write enable     | **static**                               | **DSS** | GP                                        |
| Depth compare function | **static**                               | **DSS** | GP                                        |

### Comparison 1

**Some of informations listed here are wrong**

|                      |                      Vulkan                     |                        Metal                        |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Command Queue        | `VkCommandQueue`                                | `MTLCommandQueue`                                   |
| Creation             | Retrieved from `VkDevice`                       | Created by `MTLDevice.makeQueue`                    |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Command Pool         | `VkCommandPool`                                 | no counterparts                                     |
| Creation             | Created using <`VkDevice`, queue family>        |                                                     |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Command Buffer       | `VkCommandBuffer`                               | `MTLCommandBuffer`                                  |
| Creation             | Created from `VkComamndPool`                    | Created by `MTLCommandQueue.makeCommandBuffer`      |
| Submission           | `vkQueueSubmit`                                 | `enqueue`, and then `commit`                        |
| Repeatable?          | Yes after retirement                            | ?                                                   |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Intra-Queue Sync     | `VkEvent`                                       | `MTLFence`                                          |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Inter-Queue Sync     | `VkSemaphore`                                   | no counterparts                                     |
| Creation             | `vkCreateSemaphore` on device                   |                                                     |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Host-Device Sync     | `VkFence`                                       | Callbacks of `MTLCommandBuffer`                     |
| Creation             | `vkCreateFence`                                 |                                                     |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Secondary C.B.       | `VkCommandBuffer`                               | no exact counterparts                               |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Render Pass          | `VkRenderPass`                                  | **no counterparts**                                 |
| Specifying RT        | Deferred;`vkFramebuffer` passed to              |                                                     |
|                      | `vkCmdBeginRenderPass`                          |                                                     |
| Start/End            | `vkCmdBeginRenderPass`                          |                                                     |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Render Subpass       | Encapsulated in `VkRenderPass`                  | `MTLRenderPassDescriptor`                           |
| Specifying RT        | Specified by `vkCmdBeginRenderPass`             | Immediately; specified in `MTLRenderPassDescriptor` |
|                      |                                                 |                                                     |
| Start/End            | TODO                                            | By command encoder creation                         |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Command Encoder      | Integrated into `VkCommandBuffer`               | `MTLCommandEncoder`                                 |
| Creation             | `vkBeginCommandBuffer`                          | `MTLCommandBuffer.makeRenderCommandEncoder`, etc.   |
| How many RPs?        | Contains multiple render passes                 | Contains only a **single** render pass              |
| Type Selection       | Limited by the queue family property            | Selected while creating a command encoder           |
|                      |                                                 | e.g. `makeRenderCommandEncoder` for Render          |
| Transfer/blit        | Can be done on Render/Compute queue             | Need a separate command encoder                     |
|                      |                                                 | created by `makeBlitCommandEncoder`                 |
| Completion           | `vkEndCommandBuffer`                            | `endEncoding`                                       |
| Wait for Retire      | Wait on `VkFence`                               | `waitUntilCompleted`                                |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Resource Heap        | `VkDeviceMemory`                                | `MTLHeap`  & global heap                            |
| Creation             | `vkAllocateMemory`                              | `MTLDevice.makeHeap`                                |
| Allocation           | Manual, `vkBindBufferMemory`, etc.              | Automatic, `MTLHeap.makeBuffer`, etc.               |
|                      |                                                 | Call `MTLResource.makeAliasable` to allow aliasing  |
| Memoryless Image     | Allocated by specifying the memory type         | Allocated by specifying `MTLHeapDescriptor`'s       |
|                      | with: `VK_MEMORY_PROPERTY_LAZILY_ALLOCATED_BIT` | `storageMode` to `memoryless` ()                    |
|                      |                                                 | Not just a hint; there are multiple restrictions    |
|                      |                                                 | And only supported by iOS and tvOS                  |
|                      | Bound as an input attachment                    | Bound as a read-only color attachment               |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Image                | `VkImage`                                       | `MTLTexture`                                        |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Image View           | `VkImageView`                                   | `MTLTexture`                                        |
|                      |                                                 | Created by `MTLTexture.makeTextureView`             |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Texture              |                                                 |                                                     |
| Binding              | As a member of one of descriptor sets           | In the texture argument table                       |
|                      |                                                 | `MTLCommandBuffer.setVertexTexture`, etc.           |
| Sampled Image        | DT: `VK_DESCRIPTOR_TYPE_SAMPLED_IMAGE`          | MSL: `textureX<T, access::sample>`                  |
| Storage Image        | DT: `VK_DESCRIPTOR_TYPE_STORAGE_IMAGE`          | MSL: `textureX<T, access::read_write>`              |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Texel Buffer         | `VkBufferView`                                  | **no counterparts**                                 |
| Binding              | As a member of one of descriptor sets           |                                                     |
| R/O Texel Buffer     | DT: `VK_DESCRIPTOR_TYPE_UNIFORM_TEXEL_BUFFER`   |                                                     |
| Storage Texel Buffer | DT: `VK_DESCRIPTOR_TYPE_STORAGE_TEXEL_BUFFER`   |                                                     |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Sampler              | `VkSampler`                                     | `MTLSamplerState`                                   |
| Creation             | `vkCreateSampler`                               | `MTLDevice.makeSamplerState`                        |
| Specification        |                                                 | In the sampler state argument table                 |
|                      |                                                 | `MTLCommandBuffer.setVertexSamplerState`, etc.      |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Buffer               | `VkBuffer`                                      | `MTLBuffer`                                         |
| Binding              | As a member of one of descriptor sets           | In the buffer argument table                        |
|                      |                                                 | `MTLCommandBuffer.setVertexBuffer`, etc.            |
| Dynamic Offset       | Specified at descriptor set binding time        | Updated by `setVertexBufferOffset`, etc.            |
|                      | Needs an appropriate descriptor type            |                                                     |
| (R/O) Uniform Buffer | DT: `VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER`         |                                                     |
|                      | DT: `VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER_DYNAMIC` | MSL: constant buffer                                |
| Storage Buffer       | DT: `VK_DESCRIPTOR_TYPE_STORAGE_BUFFER`         |                                                     |
|                      | DT: `VK_DESCRIPTOR_TYPE_STORAGE_BUFFER_DYNAMIC` | MSL: device buffer                                  |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Vertex Layout        | `VkVertexInputBindingDescription`               | `MTLVertexBufferLayoutDescriptor`                   |
|                      | In `VkPipelineVertexInputStateCreateInfo`       | In `MTLVertexDescriptor`                            |
| Binding              | `vkCmdBindVertexBuffers`                        | `MTLCommandBuffer.setVertexBuffer`                  |
|                      | Vertex bindings have their own table            | The same table as normal buffers                    |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
| Vertex Input         | `VkVertexInputAttributeDescription`             | `MTLVertexAttributeDescriptor`                      |
|                      | In `VkPipelineVertexInputStateCreateInfo`       | In `MTLVertexDescriptor`                            |
| Format               | `VkFormat`                                      | `MTLVertexFormat`                                   |
| -------------------- | ----------------------------------------------- | --------------------------------------------------- |
|                      |                                                 |                                                     |

TODO: memoryless texture (`VK_IMAGE_USAGE_TRANSIENT_ATTACHMENT_BIT` / `MEmoryless`)
