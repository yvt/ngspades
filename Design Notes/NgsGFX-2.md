
Nightingales GFX: 2nd Iteration
===============================

## Motivation / Goals

- Abolition of Metal-style fences for intra-engine synchronization
- Hard limit (64) on the number of pending command buffers that Metal has — might lead to a better performance if we impose the same restriction on the Vulkan backend
- Clean up the intangible API
- Full asynchronous operation of device engines
- Make the API `unsafe` by default and at the same time introduce a really safe validation layer
    - Possibility to opt out the automatic resource lifetime tracking — with the current API applications end up maintaining the lifetime of `Allocations` while `Image`s are tracked automatically
    - The current API/implementation is not actually safe (mainly because full validation is costly). Vulkan backend skips various checks. While on major Vulkan implementations it is not fatal, it is surely a specification violation and still can reault in an undefined behavior. We should not use it as an excuse for the broken safety.
    - (2018-02-20) What would the "perfectly safe" API look like? Well, first we add API call validations at our abstraction layer. For shader memory accesses, we would have to rely on the *robustness* device extension. Let's simply fail at device creation if the extension is not available.
    - But I don't want to flood the application code with `unsafe`. Instead, only the initialization code should be responsible for that
- Better typing: for the time being we are effectively abusing the associated types
    - Do we really need strict static binding? Vulkan already uses dynamic binding (indirect function calls) which has never been a performance bottleneck. Monomorphization significantly increases the compilation time.  Also the generics inhibit Racer's code auto-completion. And in various ways it makes the application code harder to read.
    - (2018-03-03) According to [this forum post](https://forums.developer.apple.com/thread/18860), an Apple GPU driver developer prototyped a non-Objective-C version of Metal and concluded that the overhead of Objective-C calls was insignificant.
- gfx-rs now comes with a low-level interface with Vulkan/Metal support. Is it still beneficial to make our own abstraction?
    - (2018-02-20) Their Metal backend barely works. https://github.com/gfx-rs/gfx/blob/master/src/backend/metal/src/command.rs I wonder if they will actually complete it...
- (2018-03-03) Recently MoltenVK was open-sourced: https://github.com/KhronosGroup/MoltenVK


## Benchmarks

### Optimizing Metal calls

Snowdash, w/o `setenv.sh`

`b1b7dde` (Mar 28, 2018):

    test cb_throughput_100                 ... bench:   3,189,822 ns/iter (+/- 1,033,351)
    test cb_throughput_200                 ... bench:   5,945,664 ns/iter (+/- 1,438,385)
    test cb_throughput_400                 ... bench:  11,103,107 ns/iter (+/- 3,424,708)
    test cmds_dispatch_10000_mt_throughput ... bench:  36,866,530 ns/iter (+/- 1,655,321)
    test cmds_dispatch_10000_throughput    ... bench:  40,634,271 ns/iter (+/- 3,274,172)

Started using `fast_msg_send`. `5da6a76` (Mar 29, 2018):

    test cb_throughput_100                 ... bench:   3,024,782 ns/iter (+/- 1,143,522)
    test cb_throughput_200                 ... bench:   5,880,300 ns/iter (+/- 2,311,251)
    test cb_throughput_400                 ... bench:  12,364,453 ns/iter (+/- 5,987,419)
    test cmds_dispatch_10000_mt_throughput ... bench:  36,140,939 ns/iter (+/- 5,180,533)
    test cmds_dispatch_10000_throughput    ... bench:  39,433,953 ns/iter (+/- 3,450,965)
