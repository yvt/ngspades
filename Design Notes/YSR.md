
YSR
===

- [Interactive Sound Propagation with Bidirectional Path Tracing](http://gaps-zju.org/bst/)
- Physically Based Real-Time Auralization of Interactive Virtual Environments
- [Digital Sound Synthesis by Physical Modeling Using the Functional ...](https://www.amazon.co.jp/Synthesis-Physical-Modeling-Functional-Transformation/dp/0306478757)
    - FTM (Functional transformation method) is better than FDM because it can neglect high frequenecy factors?
    - *Physical modeling of drums by transfer function methods*: Saved as PDF. Basics of wave field simulations. Describes each step from PDE to simulation, like the Laplace transformation and the Strum-Liouville (SL) transformation toward the computer simulation. Maybe is [this](http://www.falstad.com/circosc-java/CircOsc.java) it?
    - [Wave Field Simulation with the Functional Transformation Method](http://ieeexplore.ieee.org/document/1661276/): Saved as PDF. Presents a simulation program that divides a complex domain into multiple blocks in each of which FTM is applied. Cites some important papers. Interconnection between blocks are realized with so called port adaptors (see *Wave digital filters: ...*.)
    - *Application Of Mixed Modeling Strategies For The Simulation Of The 2D Wave Equation For Arbitrary Geometries*: Deals with arbitrary geometries comprising of multiple blocks, each of which is simulated either of FTM (for simple blocks) and FDM (for complex blocks).
    - [*Wave digital filters: Theory and practice*](http://ieeexplore.ieee.org/document/1457726/): (Access denied) Port adaptors?
    - Chapter 6 *Comparison of the FTM with the Classical Physical Modeling Methods*: : https://books.google.co.jp/books?id=cmUECAAAQBAJ&lpg=PA191&dq=low%20frequency%20acoustic%20simulation%20fdm&pg=PA189#v=onepage&q=low%20frequency%20acoustic%20simulation%20fdm&f=false 
    - *Block-Based Physical Modeling for Digital Sound Synthesis*: http://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.68.6125&rep=rep1&type=pdf Mentions port adaptors
- *Physically Based Real-Time Auralization of Interactive Virtual Environments*
    - Introduces a concept of *Diffuse Rain*
- https://github.com/reuk/parallel-reverb-raytracer
    - An implementation of acoustic ray tracer

## Sound propagation simuation

- Low frequency
    - Finite Differential Method or Functional Transformation Method
    - Want to propagate it through walls
- High frequency
    - Geometry Acoustics (GA)
    - Image Method for early reflections?
    - For late reflections, Bidirectional Path Tracing or Metropolis Path Tracing
    - Don't want to waste time on unreachable pairs - breadth-first search?
        - Save path for faster determination on next searches; listener only moves slowly. For example, construct a spanning tree from the listener position, and change the root node as the listener moves on
        - Basically, we're applying that "disconnect block detection" algorithm on the complement geometry. And in this case we only have one root node, and it can move to an adjacent cell

## Material Data

- *Auralization: Fundamentals of Acoustics, Modelling, Simulation, Algorithms and Acoustic Virtual Reality (RWTHedition)* Annex: Material Data
    - `Auralization- fundamentals of acoustics, modelling, simulation, algorithms and acoustic virtual reality - Annex- Material Data.pdf`

## Band Merging

- [Complementary N-Band IIR Filterbank Based on 2-Band Complementary Filters](http://www.iwaenc.org/proceedings/2010/HTML/Uploads/975.pdf)
- [Tree-structured complementary filter banks using all-pass sections](https://authors.library.caltech.edu/9284/1/REGieeetcs87.pdf) - up to 8 bands

## Geometric Acoustics

- [Shell: Accelerating Ray Tracing on GPU](http://www.cse.nd.edu/Reports/2011/TR-2011-03.pdf)

## Benchmarks

### `ysr2_filters::conv`

#### Initial Implementation

    test conv::tests::conv_000128_131072 ... bench:       9,182 ns/iter (+/- 1,847)
    test conv::tests::conv_001024_131072 ... bench:      68,835 ns/iter (+/- 10,226)
    test conv::tests::conv_100000_000512 ... bench:   1,198,465 ns/iter (+/- 432,282)
    test conv::tests::conv_100000_002048 ... bench:   2,478,005 ns/iter (+/- 284,864)
    test conv::tests::conv_100000_008192 ... bench:   2,928,102 ns/iter (+/- 397,254)
    test conv::tests::conv_100000_032768 ... bench:   4,246,643 ns/iter (+/- 413,308)
    test conv::tests::conv_100000_131072 ... bench:   6,869,412 ns/iter (+/- 2,527,190)
    test conv::tests::conv_100000_524288 ... bench:   7,328,707 ns/iter (+/- 1,684,468)

#### AVX + various optimizations

    test conv::tests::conv_000128_131072_1000 ... bench:   3,709,611 ns/iter (+/- 10,162,171)
    test conv::tests::conv_001024_131072_100  ... bench:   2,776,015 ns/iter (+/- 6,918,719)
    test conv::tests::conv_100000_000512      ... bench:   1,199,095 ns/iter (+/- 618,523)
    test conv::tests::conv_100000_002048      ... bench:   2,196,139 ns/iter (+/- 393,031)
    test conv::tests::conv_100000_008192      ... bench:   2,489,367 ns/iter (+/- 441,706)
    test conv::tests::conv_100000_032768      ... bench:   3,734,890 ns/iter (+/- 695,986)
    test conv::tests::conv_100000_131072      ... bench:   5,998,198 ns/iter (+/- 1,047,377)
    test conv::tests::conv_100000_524288      ... bench:   6,374,443 ns/iter (+/- 979,762)

#### Change setup to enable TDR

    test conv::tests::conv_000128_131072_1000 ... bench:   9,507,828 ns/iter (+/- 9,526,705)
    test conv::tests::conv_001024_131072_100  ... bench:   5,818,542 ns/iter (+/- 1,816,282)
    test conv::tests::conv_100000_000512      ... bench:   1,042,851 ns/iter (+/- 213,372)
    test conv::tests::conv_100000_002048      ... bench:   2,358,397 ns/iter (+/- 333,118)
    test conv::tests::conv_100000_008192      ... bench:   2,731,536 ns/iter (+/- 409,312)
    test conv::tests::conv_100000_032768      ... bench:   4,289,275 ns/iter (+/- 844,195)
    test conv::tests::conv_100000_131072      ... bench:   7,617,614 ns/iter (+/- 1,516,421)
    test conv::tests::conv_100000_524288      ... bench:   7,756,000 ns/iter (+/- 1,242,050)

#### Optimize complex multiplications

    test conv::tests::conv_000128_131072_1000 ... bench:   8,652,976 ns/iter (+/- 8,870,771)
    test conv::tests::conv_001024_131072_100  ... bench:   5,463,416 ns/iter (+/- 1,540,412)
    test conv::tests::conv_100000_000512      ... bench:     997,442 ns/iter (+/- 204,081)
    test conv::tests::conv_100000_002048      ... bench:   2,398,231 ns/iter (+/- 343,228)
    test conv::tests::conv_100000_008192      ... bench:   2,643,075 ns/iter (+/- 443,784)
    test conv::tests::conv_100000_032768      ... bench:   3,939,136 ns/iter (+/- 462,898)
    test conv::tests::conv_100000_131072      ... bench:   6,307,069 ns/iter (+/- 586,292)
    test conv::tests::conv_100000_524288      ... bench:   7,628,828 ns/iter (+/- 1,070,763)

#### yFFT: Add AVX radix-2 kernel "AvxRadix2Kernel2"

    test conv::tests::conv_000128_131072_1000 ... bench:   8,484,447 ns/iter (+/- 9,337,459)
    test conv::tests::conv_001024_131072_100  ... bench:   5,353,611 ns/iter (+/- 2,261,856)
    test conv::tests::conv_100000_000512      ... bench:     979,803 ns/iter (+/- 212,092)
    test conv::tests::conv_100000_002048      ... bench:   2,141,928 ns/iter (+/- 357,201)
    test conv::tests::conv_100000_008192      ... bench:   2,518,373 ns/iter (+/- 451,930)
    test conv::tests::conv_100000_032768      ... bench:   3,771,518 ns/iter (+/- 616,646)
    test conv::tests::conv_100000_131072      ... bench:   6,137,995 ns/iter (+/- 1,575,719)
    test conv::tests::conv_100000_524288      ... bench:   6,533,571 ns/iter (+/- 1,111,621)

### `ysr2_spatializer::bandmerger`

Initial Implemention:

    test bandmerge_lr4_100000 ... bench:  19,143,694 ns/iter (+/- 4,300,941)

YSR2: Improve the throughput of Lr4BandMerger (by roughly 40%):

    test bandmerge_lr4_100000 ... bench:  14,225,499 ns/iter (+/- 7,199,552)

YSR2: Use mul_add in BiquadKernelState:

    test bandmerge_lr4_100000 ... bench:   9,605,355 ns/iter (+/- 2,847,624)