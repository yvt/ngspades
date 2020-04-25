Nightingales Presentation Framework
===================================

## Render Tree

How should we pass render tree from UI thread to renderer?

- Use persistent data structure for the tree structure
- What about attributes (which are likely to change more often than the structure, like thousands per frame)? Separate transactional storage?

## Vector Graphics

- https://crates.io/crates/harfbuzz_rs

## Benchmarks

### Property Update

Initial implementation:

    test update_nodes_1000   ... bench:     183,606 ns/iter (+/- 155,742)
    test update_nodes_100000 ... bench:  41,779,952 ns/iter (+/- 13,581,561)

`HashMap::with_capacity`:

    test update_nodes_1000   ... bench:     104,206 ns/iter (+/- 25,629)
    test update_nodes_100000 ... bench:  43,485,209 ns/iter (+/- 8,669,781)

64-bit `ProcessUniqueId` (*NgsPF: Performance optimization*; `4a1c641`):

    test update_nodes_1000   ... bench:      95,222 ns/iter (+/- 18,230)
    test update_nodes_100000 ... bench:  36,278,617 ns/iter (+/- 7,117,491)

Introduction of `UpdateId`:

    test update_nodes_1000   ... bench:      59,300 ns/iter (+/- 21,582)
    test update_nodes_100000 ... bench:  10,993,421 ns/iter (+/- 2,158,684)

Micro-optimize `KeyedProperty` (*NgsPF: Performance optimization*; `0749569`):

    test update_nodes_1000   ... bench:      59,156 ns/iter (+/- 11,808)
    test update_nodes_100000 ... bench:  11,366,546 ns/iter (+/- 3,039,889)

### `stress`

trail len = 10, M295X (iMac 5K), `RUSTFLAGS='-Ctarget-feature=+avx,+sse3,+avx2,+fma'`

`3f7396c` (Mar 15, 2018, NgsGFX):

- Metal: 406,900 layers/sec
    - `nightly-2018-03-03-x86_64-apple-darwin`

`b1b7dde` (Mar 28, 2018, ZanGFX):

- Metal 2: 406,900 layers/sec
    - `nightly-2018-03-15-x86_64-apple-darwin`

Started using `fast_msg_send`. `5da6a76` (Mar 29, 2018, ZanGFX):

- Metal 2: 450,560 layers/sec
    - `nightly-2018-03-15-x86_64-apple-darwin`

`417e5a9` (Apr 1, 2018, ZanGFX):

- Windows Vulkan: 440,020 layers/sec
    - `nightly-2018-03-31-x86_64-apple-darwin`?

### Font Rendering (`canvas/font`)

`e00a0b6b4bb9793bc2ff06906adb20f7942264d1` (Apr 11, 2018):

    test layout_simple ... bench:      25,858 ns/iter (+/- 2,224)
    test render_text   ... bench:   1,010,093 ns/iter (+/- 85,272)

`701ded28fca90dd99b8d1260fae2ec0f92002b88`:

    test layout_simple ... bench:      26,894 ns/iter (+/- 6,722)
    test render_text   ... bench:     984,236 ns/iter (+/- 69,731)

`3cf6f457dcaf771a93bc5bec3dce2d4c33ab780a` (Apr 13, 2018): 

    test layout_simple ... bench:      25,532 ns/iter (+/- 1,577)
    test render_text   ... bench:     951,044 ns/iter (+/- 178,783)

`987d7f687ccf93055574dc93445d61b85d28eed9` (Apr 14, 2018):

    test layout_simple ... bench:      25,699 ns/iter (+/- 3,824)
    test render_text   ... bench:     303,648 ns/iter (+/- 40,868)

`605fbd449612dcdc7b00178bce01df033154e61f` (Apr 15, 2018) + `RUSTFLAGS=-Ctarget-feature=+avx,+sse3,+avx2,+fma`:

    test layout_simple ... bench:      27,256 ns/iter (+/- 5,077)
    test render_text   ... bench:     296,158 ns/iter (+/- 54,145)

`1eeb44a2eca87909a8c3387ab93900f268c39414` (Apr 15, 2018) + `RUSTFLAGS=-Ctarget-feature=+avx,+sse3,+avx2,+fma`:

    test layout_simple ... bench:      25,514 ns/iter (+/- 2,919)
    test render_text   ... bench:     272,867 ns/iter (+/- 59,845)

`71f4b9b0a4c276247ab74ed1fe1dd3c173804ddd` (Apr 15, 2018) + `RUSTFLAGS=-Ctarget-feature=+avx,+sse3,+avx2,+fma`:

    test layout_simple ... bench:      25,735 ns/iter (+/- 6,219)
    test render_text   ... bench:     258,866 ns/iter (+/- 24,495)

### Property Update (from .NET)

`70ce5d9184d8dbb93936618ab241658cfb335e86` (Apr 27, 2018):

``` ini

BenchmarkDotNet=v0.10.14, OS=macOS High Sierra 10.13.4 (17E199) [Darwin 17.5.0]
Intel Core i5-6267U CPU 2.90GHz (Skylake), 1 CPU, 4 logical and 2 physical cores
.NET Core SDK=2.1.300-preview2-008533
  [Host]     : .NET Core 2.1.0-preview2-26406-04 (CoreCLR 4.6.26406.07, CoreFX 4.6.26406.04), 64bit RyuJIT
  DefaultJob : .NET Core 2.1.0-preview2-26406-04 (CoreCLR 4.6.26406.07, CoreFX 4.6.26406.04), 64bit RyuJIT


```
|                                   Method |      Mean |    Error |   StdDev |
|----------------------------------------- |----------:|---------:|---------:|
|            SetOpacityOnMaterializedLayer |  85.52 ns | 2.161 ns | 6.338 ns |
|    SetOpacityOnMaterializedLayerWithLock |  75.32 ns | 1.505 ns | 3.040 ns |
|         SetSolidColorOnMaterializedLayer | 108.48 ns | 2.009 ns | 1.781 ns |
| SetSolidColorOnMaterializedLayerWithLock | 105.09 ns | 1.833 ns | 1.625 ns |

`fcdfc1fe28b40b3305d1f40626be233695eaa87e` (Apr 28, 2018):

``` ini

BenchmarkDotNet=v0.10.14, OS=macOS High Sierra 10.13.4 (17E199) [Darwin 17.5.0]
Intel Core i5-6267U CPU 2.90GHz (Skylake), 1 CPU, 4 logical and 2 physical cores
.NET Core SDK=2.1.300-preview2-008533
  [Host]     : .NET Core 2.1.0-preview2-26406-04 (CoreCLR 4.6.26406.07, CoreFX 4.6.26406.04), 64bit RyuJIT
  DefaultJob : .NET Core 2.1.0-preview2-26406-04 (CoreCLR 4.6.26406.07, CoreFX 4.6.26406.04), 64bit RyuJIT


```
|                                   Method |      Mean |     Error |    StdDev |
|----------------------------------------- |----------:|----------:|----------:|
|            SetOpacityOnMaterializedLayer |  93.18 ns | 1.8535 ns | 2.2064 ns |
|    SetOpacityOnMaterializedLayerWithLock |  21.31 ns | 0.4219 ns | 0.8899 ns |
|         SetSolidColorOnMaterializedLayer | 119.93 ns | 2.1235 ns | 1.8824 ns |
| SetSolidColorOnMaterializedLayerWithLock |  56.09 ns | 1.0790 ns | 0.9010 ns |

`1e81e6c2da692fde948257f936f05387dfc686b0` (Apr 28, 2018):

``` ini

BenchmarkDotNet=v0.10.14, OS=macOS High Sierra 10.13.4 (17E199) [Darwin 17.5.0]
Intel Core i5-6267U CPU 2.90GHz (Skylake), 1 CPU, 4 logical and 2 physical cores
.NET Core SDK=2.1.300-preview2-008533
  [Host]     : .NET Core 2.1.0-preview2-26406-04 (CoreCLR 4.6.26406.07, CoreFX 4.6.26406.04), 64bit RyuJIT
  DefaultJob : .NET Core 2.1.0-preview2-26406-04 (CoreCLR 4.6.26406.07, CoreFX 4.6.26406.04), 64bit RyuJIT


```
|                                   Method |      Mean |     Error |    StdDev |
|----------------------------------------- |----------:|----------:|----------:|
|            SetOpacityOnMaterializedLayer |  88.13 ns | 1.7020 ns | 1.9600 ns |
|    SetOpacityOnMaterializedLayerWithLock |  22.17 ns | 0.4416 ns | 0.4130 ns |
|         SetSolidColorOnMaterializedLayer | 122.05 ns | 2.6946 ns | 2.9950 ns |
| SetSolidColorOnMaterializedLayerWithLock |  56.32 ns | 1.0749 ns | 1.2379 ns |
