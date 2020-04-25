
BenchFFT
========

- For best results, it should be run with `nice -n -20` and other processes suspended
- `cargo bench` must be ran multiple times, and for each benchmark the result with the minimum deviation must be chosen

Python snippet to compute MFLOPS
--------------------------------

```py
import math
sizes = [2,4,8,16,32,64,128,256,512,1024,2048,4096,8192,16384]
num_fops = lambda n: n * math.log2(n) * 5
mflops =      lambda rts: [int(num_fops(size) / rt * 1000) for size, rt in zip(sizes, rts)]
mflops_real = lambda rts: [int(num_fops(size) / rt * 500) for size, rt in zip(sizes, rts)]
mflops([13, 12, 19, 33, 59, 104, 227, 434, 947, 1891, 4926, 10636, 28189, 61049])
```

     6, 25, 34, 61, 92, 169, 308, 669, 1320, 2731, 5485, 11727, 25872, 56965

Snowdash
--------

### FFTW3

                     (MFLOPS)              
    fftw3 dcif 2     1112.6   8.9877844e-09  0.001767 
    fftw3 dcif 4     3440.4   1.1626422e-08  0.001231 
    fftw3 dcif 8     5958.9   2.0138025e-08  0.001566 
    fftw3 dcif 16    7589.7   4.2162418e-08  0.001236 
    fftw3 dcif 32    9271     8.6290359e-08  0.00945 
    fftw3 dcif 64    14760    1.3007832e-07  0.025315 
    fftw3 dcif 128   22064    2.0304489e-07  0.073884 
    fftw3 dcif 256   30314    3.3779526e-07  0.151166 
    fftw3 dcif 512   37403    6.1599731e-07  0.324999 
    fftw3 dcif 1024  39829    1.2854843e-06  0.593356 
    fftw3 dcif 2048  39822    2.8285828e-06  1.13886 
    fftw3 dcif 4096  38203    6.4329834e-06  2.27029 
    fftw3 dcif 8192  34046    1.5640015e-05  4.69157 
    fftw3 dcif 16384 32349    3.5453369e-05  9.88965 
    fftw3 dcif 32768 28642    8.5803711e-05  21.6218 

### KissFFT

                       (MFLOPS)             
    kissfft dcif 2     492.1  2.0321131e-08  1.5e-05 
    kissfft dcif 4     1409.9 2.8370857e-08  1.2e-05 
    kissfft dcif 8     1234.1 9.723568e-08   1.3e-05 
    kissfft dcif 16    2438.7 1.3121891e-07  1.2e-05 
    kissfft dcif 32    1800.2 4.4438934e-07  1.5e-05 
    kissfft dcif 64    3120.3 6.153183e-07   1.5e-05 
    kissfft dcif 128   2141.8 2.0917358e-06  1.9e-05 
    kissfft dcif 256   3344.3 3.0618896e-06  2.1e-05 
    kissfft dcif 512   2566.1 8.9785156e-06  2.7e-05 
    kissfft dcif 1024  3793.3 1.3497437e-05  4.9e-05 
    kissfft dcif 2048  2750.6 4.0950928e-05  6e-05 
    kissfft dcif 4096  4013.8 6.1228516e-05  0.00012 
    kissfft dcif 8192  3019.2 0.00017636328  0.000179 
    kissfft dcif 16384 3641.8 0.00031492383  0.000332 
    kissfft dcif 32768 2840.1 0.00086533594  0.000738 

### Apple Accelerate (vDSP)

    vDSP_fft_zip
    size,mflops,time for one FFT in microseconds,mflops deviation
    2,     88.6682, 0.11278, 4.88258
    4,     359.657, 0.111217, 8.58945
    8,     943.763, 0.127151, 12.3197
    16,    4560.56, 0.0701668, 32.4108
    32,    8027.18, 0.0996613, 44.5278
    64,    14882.5, 0.12901, 204.72
    128,   14605.2, 0.30674, 263.209
    256,   20363.1, 0.502869, 306.129
    512,   24821.6, 0.928224, 291.282
    1024,  29296.7, 1.74764, 246.632
    2048,  30603.6, 3.68061, 231.192
    4096,  28286.4, 8.68826, 126.898
    8192,  26223.5, 20.3055, 316.442
    16384, 22767.2, 50.3743, 328.176

    vDSP_fft_zipt
    size,mflops,time for one FFT in microseconds,mflops deviation
    2,     88.4639, 0.11304,   1.92453
    4,     1310   , 0.0305344, 9.41408
    8,     2682.99, 0.0447263, 18.2488
    16,    4322.78, 0.0740264, 25.808
    32,    8183.79, 0.0977542, 88.0866
    64,    14874.9, 0.129077,  85.6227
    128,   14557.9, 0.307736,  126.743
    256,   20568.3, 0.497854,  133.175
    512,   25368  , 0.908231,  322.654
    1024,  29252.4, 1.75028,   269.985
    2048,  30961.3, 3.63809,   191.104
    4096,  29056.9, 8.45788,   331.805
    8192,  25688.4, 20.7284,   176.084
    16384, 25616.3, 44.7715,   228.326
    FFT Double Precision

```py
# computes BenchFFT MFLOPS
[(x[0], 5000*x[0]*math.log2(x[0])/x[1]) for x in zip([2**x for x in range(0,15)], [1, 8, 33, 77, 179, 404, 900, 2170, 4575, 10436, 22987, 52986, 109806, 243783, 470251])]

# short form
[int(5000*x[0]*math.log2(x[0])/x[1]) for x in zip([2**x for x in range(0,15)], [1, 8, 33, 77, 179, 404, 900, 2170, 4575, 10436, 22987, 52986, 109806, 243783, 470251])]
```

### yFFT 5b52239 (April 13, 2017)

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)       
    test tests::simple_benchmark_00002 ... bench:          95 ns/iter (+/- 55)          105 MFLOPS
    test tests::simple_benchmark_00004 ... bench:         173 ns/iter (+/- 78)          231 MFLOPS
    test tests::simple_benchmark_00008 ... bench:         446 ns/iter (+/- 245)         269 MFLOPS
    test tests::simple_benchmark_00016 ... bench:         949 ns/iter (+/- 348)         337 MFLOPS
    test tests::simple_benchmark_00032 ... bench:       2,575 ns/iter (+/- 486)         310 MFLOPS
    test tests::simple_benchmark_00064 ... bench:       5,488 ns/iter (+/- 867)         349 MFLOPS
    test tests::simple_benchmark_00128 ... bench:      12,392 ns/iter (+/- 5,706)       361 MFLOPS
    test tests::simple_benchmark_00256 ... bench:      26,653 ns/iter (+/- 16,356)      384 MFLOPS
    test tests::simple_benchmark_00512 ... bench:      59,049 ns/iter (+/- 22,335)      390 MFLOPS
    test tests::simple_benchmark_01024 ... bench:     129,609 ns/iter (+/- 41,118)      395 MFLOPS
    test tests::simple_benchmark_02048 ... bench:     286,186 ns/iter (+/- 97,587)      393 MFLOPS
    test tests::simple_benchmark_04096 ... bench:     652,194 ns/iter (+/- 111,072)     376 MFLOPS
    test tests::simple_benchmark_08192 ... bench:   1,332,517 ns/iter (+/- 384,396)     399 MFLOPS
    test tests::simple_benchmark_16384 ... bench:   2,900,888 ns/iter (+/- 373,295)     395 MFLOPS

### yFFT: Subtle optimization for generic kernels (6f31b42):

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:          45 ns/iter (+/- 30)
    test tests::simple_benchmark_00004 ... bench:         116 ns/iter (+/- 46)
    test tests::simple_benchmark_00008 ... bench:         346 ns/iter (+/- 145)
    test tests::simple_benchmark_00016 ... bench:         852 ns/iter (+/- 543)
    test tests::simple_benchmark_00032 ... bench:       2,329 ns/iter (+/- 361)
    test tests::simple_benchmark_00064 ... bench:       4,918 ns/iter (+/- 1,843)
    test tests::simple_benchmark_00128 ... bench:      11,583 ns/iter (+/- 3,732)
    test tests::simple_benchmark_00256 ... bench:      27,852 ns/iter (+/- 5,456)
    test tests::simple_benchmark_00512 ... bench:      61,166 ns/iter (+/- 43,817)
    test tests::simple_benchmark_01024 ... bench:     137,361 ns/iter (+/- 22,478)
    test tests::simple_benchmark_02048 ... bench:     303,680 ns/iter (+/- 48,406)
    test tests::simple_benchmark_04096 ... bench:     615,274 ns/iter (+/- 298,516)
    test tests::simple_benchmark_08192 ... bench:   1,412,824 ns/iter (+/- 191,120)
    test tests::simple_benchmark_16384 ... bench:   2,918,528 ns/iter (+/- 398,214)

### yFFT: Add Radix-2/4 generic kernels (5b6ad53): 

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:          18 ns/iter (+/- 3)            555 MFLOPS
    test tests::simple_benchmark_00004 ... bench:          33 ns/iter (+/- 18)          1212 MFLOPS
    test tests::simple_benchmark_00008 ... bench:          77 ns/iter (+/- 36)          1558 MFLOPS
    test tests::simple_benchmark_00016 ... bench:         179 ns/iter (+/- 27)          1787 MFLOPS
    test tests::simple_benchmark_00032 ... bench:         404 ns/iter (+/- 325)         1980 MFLOPS
    test tests::simple_benchmark_00064 ... bench:         900 ns/iter (+/- 179)         2133 MFLOPS
    test tests::simple_benchmark_00128 ... bench:       2,170 ns/iter (+/- 427)         2064 MFLOPS
    test tests::simple_benchmark_00256 ... bench:       4,575 ns/iter (+/- 3,641)       2238 MFLOPS
    test tests::simple_benchmark_00512 ... bench:      10,436 ns/iter (+/- 2,361)       2207 MFLOPS
    test tests::simple_benchmark_01024 ... bench:      22,987 ns/iter (+/- 4,969)       2227 MFLOPS
    test tests::simple_benchmark_02048 ... bench:      52,986 ns/iter (+/- 13,239)      2125 MFLOPS
    test tests::simple_benchmark_04096 ... bench:     109,806 ns/iter (+/- 21,577)      2238 MFLOPS
    test tests::simple_benchmark_08192 ... bench:     243,783 ns/iter (+/- 37,675)      2184 MFLOPS
    test tests::simple_benchmark_16384 ... bench:     470,251 ns/iter (+/- 178,616)     2438 MFLOPS

### yFFT: Add ability to disable bounds checking for moar performance (6b44623):

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:          17 ns/iter (+/- 1)            588 MFLOPS
    test tests::simple_benchmark_00004 ... bench:          29 ns/iter (+/- 4)           1379 MFLOPS
    test tests::simple_benchmark_00008 ... bench:          64 ns/iter (+/- 10)          1875 MFLOPS
    test tests::simple_benchmark_00016 ... bench:         124 ns/iter (+/- 17)          2580 MFLOPS
    test tests::simple_benchmark_00032 ... bench:         285 ns/iter (+/- 135)         2807 MFLOPS
    test tests::simple_benchmark_00064 ... bench:         597 ns/iter (+/- 63)          3216 MFLOPS
    test tests::simple_benchmark_00128 ... bench:       1,399 ns/iter (+/- 240)         3202 MFLOPS
    test tests::simple_benchmark_00256 ... bench:       2,985 ns/iter (+/- 341)         3430 MFLOPS
    test tests::simple_benchmark_00512 ... bench:       6,912 ns/iter (+/- 984)         3333 MFLOPS
    test tests::simple_benchmark_01024 ... bench:      13,697 ns/iter (+/- 5,871)       3738 MFLOPS
    test tests::simple_benchmark_02048 ... bench:      33,204 ns/iter (+/- 3,978)       3392 MFLOPS
    test tests::simple_benchmark_04096 ... bench:      71,730 ns/iter (+/- 14,926)      3426 MFLOPS
    test tests::simple_benchmark_08192 ... bench:     155,070 ns/iter (+/- 23,981)      3433 MFLOPS
    test tests::simple_benchmark_16384 ... bench:     328,711 ns/iter (+/- 59,002)      3489 MFLOPS

### yFFT: Use SliceAccessor in the bit reversal kernel (81c83ff):

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:          15 ns/iter (+/- 10)           666 MFLOPS
    test tests::simple_benchmark_00004 ... bench:          23 ns/iter (+/- 16)          1739 MFLOPS
    test tests::simple_benchmark_00008 ... bench:          55 ns/iter (+/- 9)           2181 MFLOPS
    test tests::simple_benchmark_00016 ... bench:         108 ns/iter (+/- 20)          2962 MFLOPS
    test tests::simple_benchmark_00032 ... bench:         247 ns/iter (+/- 189)         3238 MFLOPS
    test tests::simple_benchmark_00064 ... bench:         534 ns/iter (+/- 122)         3595 MFLOPS
    test tests::simple_benchmark_00128 ... bench:       1,307 ns/iter (+/- 266)         3427 MFLOPS
    test tests::simple_benchmark_00256 ... bench:       2,830 ns/iter (+/- 475)         3618 MFLOPS
    test tests::simple_benchmark_00512 ... bench:       6,439 ns/iter (+/- 1,249)       3578 MFLOPS
    test tests::simple_benchmark_01024 ... bench:      13,435 ns/iter (+/- 2,176)       3810 MFLOPS
    test tests::simple_benchmark_02048 ... bench:      30,510 ns/iter (+/- 5,850)       3691 MFLOPS
    test tests::simple_benchmark_04096 ... bench:      63,797 ns/iter (+/- 12,645)      3852 MFLOPS
    test tests::simple_benchmark_08192 ... bench:     147,219 ns/iter (+/- 24,783)      3616 MFLOPS
    test tests::simple_benchmark_16384 ... bench:     287,222 ns/iter (+/- 40,404)      3993 MFLOPS

### yFFT: Optimize x86 SSE DIT kernel (c5a0375) **NOT BENCHFFT COMPLIANT**:

    test tests::dit_benchmark_00064    ... bench:         286 ns/iter (+/- 54)
    test tests::dit_benchmark_00256    ... bench:       1,335 ns/iter (+/- 190)
    test tests::dit_benchmark_00512    ... bench:       3,006 ns/iter (+/- 568)
    test tests::dit_benchmark_02048    ... bench:      13,526 ns/iter (+/- 7,574)
    test tests::dit_benchmark_08192    ... bench:      67,744 ns/iter (+/- 9,507)

### yFFT: Optimize x86 SSE DIT kernel (85e2696) **NOT BENCHFFT COMPLIANT**:

    test tests::dit_benchmark_00064    ... bench:         246 ns/iter (+/- 50)
    test tests::dit_benchmark_00256    ... bench:       1,203 ns/iter (+/- 257)
    test tests::dit_benchmark_00512    ... bench:       2,672 ns/iter (+/- 347)
    test tests::dit_benchmark_02048    ... bench:      12,590 ns/iter (+/- 1,742)
    test tests::dit_benchmark_08192    ... bench:      59,032 ns/iter (+/- 9,943)

### yFFT: Implement x86 Radix-2 DIF SSE kernel (98962f5):

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:          13 ns/iter (+/- 7)            769 MFLOPS
    test tests::simple_benchmark_00004 ... bench:          22 ns/iter (+/- 12)          1818 MFLOPS
    test tests::simple_benchmark_00008 ... bench:          42 ns/iter (+/- 8)           2857 MFLOPS
    test tests::simple_benchmark_00016 ... bench:          75 ns/iter (+/- 13)          4266 MFLOPS
    test tests::simple_benchmark_00032 ... bench:         158 ns/iter (+/- 33)          5063 MFLOPS
    test tests::simple_benchmark_00064 ... bench:         325 ns/iter (+/- 64)          5907 MFLOPS
    test tests::simple_benchmark_00128 ... bench:         668 ns/iter (+/- 128)         6706 MFLOPS
    test tests::simple_benchmark_00256 ... bench:       1,366 ns/iter (+/- 473)         7496 MFLOPS
    test tests::simple_benchmark_00512 ... bench:       3,225 ns/iter (+/- 564)         7144 MFLOPS
    test tests::simple_benchmark_01024 ... bench:       6,732 ns/iter (+/- 984)         7605 MFLOPS
    test tests::simple_benchmark_02048 ... bench:      14,694 ns/iter (+/- 2,591)       7665 MFLOPS
    test tests::simple_benchmark_04096 ... bench:      32,881 ns/iter (+/- 5,223)       7474 MFLOPS
    test tests::simple_benchmark_08192 ... bench:      71,287 ns/iter (+/- 12,688)      7469 MFLOPS
    test tests::simple_benchmark_16384 ... bench:     149,562 ns/iter (+/- 26,915)      7668 MFLOPS

### yFFT: Optimize x86 Radix-2 SSE kernel further (b778aaa):

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:          12 ns/iter (+/- 6)            833 MFLOPS
    test tests::simple_benchmark_00004 ... bench:          23 ns/iter (+/- 3)           1739 MFLOPS
    test tests::simple_benchmark_00008 ... bench:          39 ns/iter (+/- 19)          3076 MFLOPS
    test tests::simple_benchmark_00016 ... bench:          70 ns/iter (+/- 18)          4571 MFLOPS
    test tests::simple_benchmark_00032 ... bench:         149 ns/iter (+/- 25)          5369 MFLOPS
    test tests::simple_benchmark_00064 ... bench:         303 ns/iter (+/- 68)          6336 MFLOPS
    test tests::simple_benchmark_00128 ... bench:         610 ns/iter (+/- 105)         7344 MFLOPS
    test tests::simple_benchmark_00256 ... bench:       1,323 ns/iter (+/- 239)         7739 MFLOPS
    test tests::simple_benchmark_00512 ... bench:       2,940 ns/iter (+/- 523)         7836 MFLOPS
    test tests::simple_benchmark_01024 ... bench:       6,148 ns/iter (+/- 1,090)       8327 MFLOPS
    test tests::simple_benchmark_02048 ... bench:      12,864 ns/iter (+/- 2,184)       8756 MFLOPS
    test tests::simple_benchmark_04096 ... bench:      28,901 ns/iter (+/- 5,903)       8503 MFLOPS
    test tests::simple_benchmark_08192 ... bench:      63,836 ns/iter (+/- 15,459)      8341 MFLOPS
    test tests::simple_benchmark_16384 ... bench:     143,974 ns/iter (+/- 30,536)      7965 MFLOPS

### yFFT: Add x86 SSE Radix-4 kernel (wip) (e8d7a99):

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 1)
    test tests::simple_benchmark_00002 ... bench:          13 ns/iter (+/- 2)            769 MFLOPS
    test tests::simple_benchmark_00004 ... bench:          24 ns/iter (+/- 3)           1666 MFLOPS
    test tests::simple_benchmark_00008 ... bench:          40 ns/iter (+/- 15)          3000 MFLOPS
    test tests::simple_benchmark_00016 ... bench:          66 ns/iter (+/- 8)           4848 MFLOPS
    test tests::simple_benchmark_00032 ... bench:         125 ns/iter (+/- 22)          6400 MFLOPS
    test tests::simple_benchmark_00064 ... bench:         262 ns/iter (+/- 43)          7328 MFLOPS
    test tests::simple_benchmark_00128 ... bench:         547 ns/iter (+/- 145)         8190 MFLOPS
    test tests::simple_benchmark_00256 ... bench:       1,051 ns/iter (+/- 190)         9743 MFLOPS
    test tests::simple_benchmark_00512 ... bench:       2,309 ns/iter (+/- 472)         9978 MFLOPS
    test tests::simple_benchmark_01024 ... bench:       4,763 ns/iter (+/- 979)        10749 MFLOPS
    test tests::simple_benchmark_02048 ... bench:      10,615 ns/iter (+/- 2,925)      10611 MFLOPS
    test tests::simple_benchmark_04096 ... bench:      21,366 ns/iter (+/- 9,321)      11502 MFLOPS
    test tests::simple_benchmark_08192 ... bench:      52,448 ns/iter (+/- 13,617)     10152 MFLOPS
    test tests::simple_benchmark_16384 ... bench:     108,525 ns/iter (+/- 19,856)     10567 MFLOPS

### yFFT: Implement more x86 SSE radix-4 kernels (25a0a50):

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:          12 ns/iter (+/- 6)            833 MFLOPS
    test tests::simple_benchmark_00004 ... bench:          16 ns/iter (+/- 2)           2500 MFLOPS
    test tests::simple_benchmark_00008 ... bench:          33 ns/iter (+/- 6)           3636 MFLOPS
    test tests::simple_benchmark_00016 ... bench:          44 ns/iter (+/- 16)          7272 MFLOPS
    test tests::simple_benchmark_00032 ... bench:         108 ns/iter (+/- 19)          7407 MFLOPS
    test tests::simple_benchmark_00064 ... bench:         186 ns/iter (+/- 87)         10322 MFLOPS
    test tests::simple_benchmark_00128 ... bench:         425 ns/iter (+/- 201)        10541 MFLOPS
    test tests::simple_benchmark_00256 ... bench:         784 ns/iter (+/- 101)        13061 MFLOPS
    test tests::simple_benchmark_00512 ... bench:       2,053 ns/iter (+/- 237)        11222 MFLOPS
    test tests::simple_benchmark_01024 ... bench:       3,793 ns/iter (+/- 600)        13498 MFLOPS
    test tests::simple_benchmark_02048 ... bench:       9,168 ns/iter (+/- 1,627)      12286 MFLOPS
    test tests::simple_benchmark_04096 ... bench:      18,362 ns/iter (+/- 2,497)      13384 MFLOPS
    test tests::simple_benchmark_08192 ... bench:      47,085 ns/iter (+/- 8,210)      11308 MFLOPS
    test tests::simple_benchmark_16384 ... bench:      91,768 ns/iter (+/- 14,958)     12497 MFLOPS

### yFFT: Optimize bit reversal kernel (6291907):

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:          13 ns/iter (+/- 2)            769 MFLOPS
    test tests::simple_benchmark_00004 ... bench:          13 ns/iter (+/- 2)           3076 MFLOPS
    test tests::simple_benchmark_00008 ... bench:          30 ns/iter (+/- 4)           4000 MFLOPS
    test tests::simple_benchmark_00016 ... bench:          41 ns/iter (+/- 18)          7804 MFLOPS
    test tests::simple_benchmark_00032 ... bench:          96 ns/iter (+/- 17)          8333 MFLOPS
    test tests::simple_benchmark_00064 ... bench:         152 ns/iter (+/- 25)         12631 MFLOPS
    test tests::simple_benchmark_00128 ... bench:         383 ns/iter (+/- 64)         11697 MFLOPS
    test tests::simple_benchmark_00256 ... bench:         661 ns/iter (+/- 143)        15491 MFLOPS
    test tests::simple_benchmark_00512 ... bench:       1,784 ns/iter (+/- 360)        12914 MFLOPS
    test tests::simple_benchmark_01024 ... bench:       3,225 ns/iter (+/- 711)        15875 MFLOPS
    test tests::simple_benchmark_02048 ... bench:       7,919 ns/iter (+/- 2,721)      14224 MFLOPS
    test tests::simple_benchmark_04096 ... bench:      16,787 ns/iter (+/- 2,999)      14639 MFLOPS
    test tests::simple_benchmark_08192 ... bench:      44,853 ns/iter (+/- 10,781)     11871 MFLOPS
    test tests::simple_benchmark_16384 ... bench:      85,977 ns/iter (+/- 15,895)     13339 MFLOPS
        
### yFFT: Add SSE3 Radix-4 kernel (dc6f834):

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:          13 ns/iter (+/- 2)            769 MFLOPS
    test tests::simple_benchmark_00004 ... bench:          13 ns/iter (+/- 3)           3076 MFLOPS
    test tests::simple_benchmark_00008 ... bench:          28 ns/iter (+/- 3)           4285 MFLOPS
    test tests::simple_benchmark_00016 ... bench:          39 ns/iter (+/- 11)          8205 MFLOPS
    test tests::simple_benchmark_00032 ... bench:          91 ns/iter (+/- 10)          8791 MFLOPS
    test tests::simple_benchmark_00064 ... bench:         151 ns/iter (+/- 24)         12715 MFLOPS
    test tests::simple_benchmark_00128 ... bench:         356 ns/iter (+/- 137)        12584 MFLOPS
    test tests::simple_benchmark_00256 ... bench:         640 ns/iter (+/- 211)        16000 MFLOPS
    test tests::simple_benchmark_00512 ... bench:       1,659 ns/iter (+/- 194)        13887 MFLOPS
    test tests::simple_benchmark_01024 ... bench:       3,172 ns/iter (+/- 503)        16141 MFLOPS
    test tests::simple_benchmark_02048 ... bench:       7,975 ns/iter (+/- 1,005)      14124 MFLOPS
    test tests::simple_benchmark_04096 ... bench:      16,427 ns/iter (+/- 2,280)      14960 MFLOPS
    test tests::simple_benchmark_08192 ... bench:      41,896 ns/iter (+/- 12,363)     12709 MFLOPS
    test tests::simple_benchmark_16384 ... bench:      85,562 ns/iter (+/- 15,255)     13404 MFLOPS

### With `RUSTFLAGS='-Ctarget-feature=+avx` enabled:

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:          13 ns/iter (+/- 2)
    test tests::simple_benchmark_00004 ... bench:          12 ns/iter (+/- 3)
    test tests::simple_benchmark_00008 ... bench:          29 ns/iter (+/- 4)
    test tests::simple_benchmark_00016 ... bench:          39 ns/iter (+/- 20)
    test tests::simple_benchmark_00032 ... bench:          90 ns/iter (+/- 13)
    test tests::simple_benchmark_00064 ... bench:         150 ns/iter (+/- 24)
    test tests::simple_benchmark_00128 ... bench:         365 ns/iter (+/- 49)
    test tests::simple_benchmark_00256 ... bench:         619 ns/iter (+/- 87)
    test tests::simple_benchmark_00512 ... bench:       1,673 ns/iter (+/- 224)
    test tests::simple_benchmark_01024 ... bench:       3,051 ns/iter (+/- 468)
    test tests::simple_benchmark_02048 ... bench:       8,034 ns/iter (+/- 1,588)
    test tests::simple_benchmark_04096 ... bench:      15,819 ns/iter (+/- 3,094)
    test tests::simple_benchmark_08192 ... bench:      42,800 ns/iter (+/- 9,972)
    test tests::simple_benchmark_16384 ... bench:      86,961 ns/iter (+/- 17,221)

### Add `AvxRadix4Kernel4` (---):

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:          13 ns/iter (+/- 2)             769 MFLOPS
    test tests::simple_benchmark_00004 ... bench:          12 ns/iter (+/- 1)            3333 MFLOPS
    test tests::simple_benchmark_00008 ... bench:          29 ns/iter (+/- 4)            4137 MFLOPS
    test tests::simple_benchmark_00016 ... bench:          41 ns/iter (+/- 5)            7804 MFLOPS
    test tests::simple_benchmark_00032 ... bench:          82 ns/iter (+/- 15)           9756 MFLOPS
    test tests::simple_benchmark_00064 ... bench:         135 ns/iter (+/- 25)          14222 MFLOPS
    test tests::simple_benchmark_00128 ... bench:         317 ns/iter (+/- 63)          14132 MFLOPS
    test tests::simple_benchmark_00256 ... bench:         499 ns/iter (+/- 103)         20521 MFLOPS
    test tests::simple_benchmark_00512 ... bench:       1,274 ns/iter (+/- 224)         18084 MFLOPS
    test tests::simple_benchmark_01024 ... bench:       2,302 ns/iter (+/- 431)         22241 MFLOPS
    test tests::simple_benchmark_02048 ... bench:       5,936 ns/iter (+/- 1,323)       18975 MFLOPS
    test tests::simple_benchmark_04096 ... bench:      11,921 ns/iter (+/- 2,016)       20615 MFLOPS
    test tests::simple_benchmark_08192 ... bench:      33,060 ns/iter (+/- 7,038)       16106 MFLOPS
    test tests::simple_benchmark_16384 ... bench:      69,093 ns/iter (+/- 10,929)      16599 MFLOPS

### Add `AvxRadix4Kernel3` (---):

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:          13 ns/iter (+/- 2)             769 MFLOPS
    test tests::simple_benchmark_00004 ... bench:          12 ns/iter (+/- 1)            3333 MFLOPS
    test tests::simple_benchmark_00008 ... bench:          29 ns/iter (+/- 7)            4137 MFLOPS
    test tests::simple_benchmark_00016 ... bench:          34 ns/iter (+/- 3)            9411 MFLOPS
    test tests::simple_benchmark_00032 ... bench:          82 ns/iter (+/- 8)            9756 MFLOPS
    test tests::simple_benchmark_00064 ... bench:         117 ns/iter (+/- 19)          16410 MFLOPS
    test tests::simple_benchmark_00128 ... bench:         316 ns/iter (+/- 70)          14177 MFLOPS
    test tests::simple_benchmark_00256 ... bench:         457 ns/iter (+/- 48)          22407 MFLOPS
    test tests::simple_benchmark_00512 ... bench:       1,234 ns/iter (+/- 190)         18670 MFLOPS
    test tests::simple_benchmark_01024 ... bench:       1,983 ns/iter (+/- 358)         25819 MFLOPS
    test tests::simple_benchmark_02048 ... bench:       5,747 ns/iter (+/- 618)         19599 MFLOPS
    test tests::simple_benchmark_04096 ... bench:      10,940 ns/iter (+/- 2,440)       22464 MFLOPS
    test tests::simple_benchmark_08192 ... bench:      31,220 ns/iter (+/- 4,954)       17055 MFLOPS
    test tests::simple_benchmark_16384 ... bench:      62,603 ns/iter (+/- 17,858)      18319 MFLOPS
 
### yFFT: Add some AVX kernels (5c47124): 

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:          12 ns/iter (+/- 7)             833 MFLOPS
    test tests::simple_benchmark_00004 ... bench:          12 ns/iter (+/- 6)            3333 MFLOPS
    test tests::simple_benchmark_00008 ... bench:          28 ns/iter (+/- 6)            4285 MFLOPS
    test tests::simple_benchmark_00016 ... bench:          34 ns/iter (+/- 6)            9411 MFLOPS
    test tests::simple_benchmark_00032 ... bench:          76 ns/iter (+/- 14)          10526 MFLOPS
    test tests::simple_benchmark_00064 ... bench:         121 ns/iter (+/- 23)          15867 MFLOPS
    test tests::simple_benchmark_00128 ... bench:         276 ns/iter (+/- 52)          16231 MFLOPS
    test tests::simple_benchmark_00256 ... bench:         435 ns/iter (+/- 170)         23540 MFLOPS
    test tests::simple_benchmark_00512 ... bench:       1,179 ns/iter (+/- 200)         19541 MFLOPS
    test tests::simple_benchmark_01024 ... bench:       2,024 ns/iter (+/- 307)         25296 MFLOPS
    test tests::simple_benchmark_02048 ... bench:       5,149 ns/iter (+/- 991)         21876 MFLOPS
    test tests::simple_benchmark_04096 ... bench:      10,989 ns/iter (+/- 2,014)       22364 MFLOPS
    test tests::simple_benchmark_08192 ... bench:      29,006 ns/iter (+/- 5,050)       18357 MFLOPS
    test tests::simple_benchmark_16384 ... bench:      62,390 ns/iter (+/- 13,144)      18382 MFLOPS
    
### Without bit reversal **NOT BENCHFFT COMPLIANT**:

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:           5 ns/iter (+/- 0)
    test tests::simple_benchmark_00004 ... bench:           5 ns/iter (+/- 1)
    test tests::simple_benchmark_00008 ... bench:          18 ns/iter (+/- 2)
    test tests::simple_benchmark_00016 ... bench:          20 ns/iter (+/- 18)
    test tests::simple_benchmark_00032 ... bench:          52 ns/iter (+/- 7)
    test tests::simple_benchmark_00064 ... bench:          69 ns/iter (+/- 11)
    test tests::simple_benchmark_00128 ... bench:         207 ns/iter (+/- 33)
    test tests::simple_benchmark_00256 ... bench:         326 ns/iter (+/- 52)
    test tests::simple_benchmark_00512 ... bench:         870 ns/iter (+/- 114)
    test tests::simple_benchmark_01024 ... bench:       1,460 ns/iter (+/- 270)
    test tests::simple_benchmark_02048 ... bench:       3,914 ns/iter (+/- 570)
    test tests::simple_benchmark_04096 ... bench:       7,419 ns/iter (+/- 1,065)
    test tests::simple_benchmark_08192 ... bench:      20,580 ns/iter (+/- 3,716)
    test tests::simple_benchmark_16384 ... bench:      37,854 ns/iter (+/- 7,327)

### Add `AvxRadix4Kernel2` (-------):

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:          13 ns/iter (+/- 2)             769 MFLOPS
    test tests::simple_benchmark_00004 ... bench:          12 ns/iter (+/- 4)            3333 MFLOPS
    test tests::simple_benchmark_00008 ... bench:          30 ns/iter (+/- 3)            4000 MFLOPS
    test tests::simple_benchmark_00016 ... bench:          37 ns/iter (+/- 8)            8648 MFLOPS
    test tests::simple_benchmark_00032 ... bench:          70 ns/iter (+/- 12)          11428 MFLOPS
    test tests::simple_benchmark_00064 ... bench:         120 ns/iter (+/- 23)          16000 MFLOPS
    test tests::simple_benchmark_00128 ... bench:         253 ns/iter (+/- 67)          17707 MFLOPS
    test tests::simple_benchmark_00256 ... bench:         469 ns/iter (+/- 87)          21833 MFLOPS
    test tests::simple_benchmark_00512 ... bench:       1,070 ns/iter (+/- 200)         21532 MFLOPS
    test tests::simple_benchmark_01024 ... bench:       2,090 ns/iter (+/- 289)         24497 MFLOPS
    test tests::simple_benchmark_02048 ... bench:       5,022 ns/iter (+/- 1,008)       22429 MFLOPS
    test tests::simple_benchmark_04096 ... bench:      11,073 ns/iter (+/- 1,729)       22194 MFLOPS
    test tests::simple_benchmark_08192 ... bench:      29,240 ns/iter (+/- 4,974)       18210 MFLOPS
    test tests::simple_benchmark_16384 ... bench:      64,156 ns/iter (+/- 16,484)      17876 MFLOPS

### yFFT: Add more AVX kernels (273ae81):

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:          13 ns/iter (+/- 2)             769 MFLOPS
    test tests::simple_benchmark_00004 ... bench:          12 ns/iter (+/- 1)            3333 MFLOPS
    test tests::simple_benchmark_00008 ... bench:          23 ns/iter (+/- 8)            5217 MFLOPS
    test tests::simple_benchmark_00016 ... bench:          38 ns/iter (+/- 8)            8421 MFLOPS
    test tests::simple_benchmark_00032 ... bench:          63 ns/iter (+/- 34)          12698 MFLOPS
    test tests::simple_benchmark_00064 ... bench:         112 ns/iter (+/- 31)          17142 MFLOPS
    test tests::simple_benchmark_00128 ... bench:         228 ns/iter (+/- 127)         19649 MFLOPS
    test tests::simple_benchmark_00256 ... bench:         442 ns/iter (+/- 53)          23167 MFLOPS
    test tests::simple_benchmark_00512 ... bench:       1,002 ns/iter (+/- 206)         22994 MFLOPS
    test tests::simple_benchmark_01024 ... bench:       1,984 ns/iter (+/- 300)         25806 MFLOPS
    test tests::simple_benchmark_02048 ... bench:       5,022 ns/iter (+/- 796)         22429 MFLOPS
    test tests::simple_benchmark_04096 ... bench:      11,033 ns/iter (+/- 1,371)       22274 MFLOPS
    test tests::simple_benchmark_08192 ... bench:      28,348 ns/iter (+/- 5,780)       18783 MFLOPS
    test tests::simple_benchmark_16384 ... bench:      63,535 ns/iter (+/- 11,590)      18051 MFLOPS

### Simulation: no redundant copy in bit reversal **NOT REAL FFT**:

    test tests::simple_benchmark_00001 ... bench:           1 ns/iter (+/- 0)
    test tests::simple_benchmark_00002 ... bench:          13 ns/iter (+/- 6)             769 MFLOPS
    test tests::simple_benchmark_00004 ... bench:          12 ns/iter (+/- 1)            3333 MFLOPS
    test tests::simple_benchmark_00008 ... bench:          19 ns/iter (+/- 2)            6315 MFLOPS
    test tests::simple_benchmark_00016 ... bench:          33 ns/iter (+/- 5)            9696 MFLOPS
    test tests::simple_benchmark_00032 ... bench:          59 ns/iter (+/- 14)          13559 MFLOPS
    test tests::simple_benchmark_00064 ... bench:         104 ns/iter (+/- 103)         18461 MFLOPS
    test tests::simple_benchmark_00128 ... bench:         227 ns/iter (+/- 29)          19735 MFLOPS
    test tests::simple_benchmark_00256 ... bench:         434 ns/iter (+/- 73)          23594 MFLOPS
    test tests::simple_benchmark_00512 ... bench:         947 ns/iter (+/- 807)         24329 MFLOPS
    test tests::simple_benchmark_01024 ... bench:       1,891 ns/iter (+/- 634)         27075 MFLOPS
    test tests::simple_benchmark_02048 ... bench:       4,926 ns/iter (+/- 834)         22866 MFLOPS
    test tests::simple_benchmark_04096 ... bench:      10,636 ns/iter (+/- 2,281)       23106 MFLOPS
    test tests::simple_benchmark_08192 ... bench:      28,189 ns/iter (+/- 6,988)       18889 MFLOPS
    test tests::simple_benchmark_16384 ... bench:      61,049 ns/iter (+/- 8,074)       18786 MFLOPS
 

### yFFT: Add pure real sequence FFT (ece820e):

    test benchmark::simple_benchmark_real_00002 ... bench:           6 ns/iter (+/- 1)         833 MFLOPS
    test benchmark::simple_benchmark_real_00004 ... bench:          25 ns/iter (+/- 5)         800 MFLOPS
    test benchmark::simple_benchmark_real_00008 ... bench:          34 ns/iter (+/- 7)        1764 MFLOPS
    test benchmark::simple_benchmark_real_00016 ... bench:          61 ns/iter (+/- 10)       2622 MFLOPS
    test benchmark::simple_benchmark_real_00032 ... bench:          92 ns/iter (+/- 21)       4347 MFLOPS
    test benchmark::simple_benchmark_real_00064 ... bench:         169 ns/iter (+/- 90)       5680 MFLOPS
    test benchmark::simple_benchmark_real_00128 ... bench:         308 ns/iter (+/- 91)       7272 MFLOPS
    test benchmark::simple_benchmark_real_00256 ... bench:         669 ns/iter (+/- 211)      7653 MFLOPS
    test benchmark::simple_benchmark_real_00512 ... bench:       1,320 ns/iter (+/- 297)      8727 MFLOPS
    test benchmark::simple_benchmark_real_01024 ... bench:       2,731 ns/iter (+/- 925)      9373 MFLOPS
    test benchmark::simple_benchmark_real_02048 ... bench:       5,485 ns/iter (+/- 2,293)   10268 MFLOPS
    test benchmark::simple_benchmark_real_04096 ... bench:      11,727 ns/iter (+/- 2,529)   10478 MFLOPS
    test benchmark::simple_benchmark_real_08192 ... bench:      25,872 ns/iter (+/- 20,162)  10290 MFLOPS
    test benchmark::simple_benchmark_real_16384 ... bench:      56,965 ns/iter (+/- 12,147)  10066 MFLOPS

### yFFT: Add SSE real sequence kernel:

    test benchmark::simple_benchmark_real_00002 ... bench:           6 ns/iter (+/- 0)         833 MFLOPS
    test benchmark::simple_benchmark_real_00004 ... bench:          26 ns/iter (+/- 6)         769 MFLOPS
    test benchmark::simple_benchmark_real_00008 ... bench:          33 ns/iter (+/- 6)        1818 MFLOPS
    test benchmark::simple_benchmark_real_00016 ... bench:          51 ns/iter (+/- 15)       3137 MFLOPS
    test benchmark::simple_benchmark_real_00032 ... bench:          70 ns/iter (+/- 16)       5714 MFLOPS
    test benchmark::simple_benchmark_real_00064 ... bench:         136 ns/iter (+/- 19)       7058 MFLOPS
    test benchmark::simple_benchmark_real_00128 ... bench:         242 ns/iter (+/- 84)       9256 MFLOPS
    test benchmark::simple_benchmark_real_00256 ... bench:         517 ns/iter (+/- 81)       9903 MFLOPS
    test benchmark::simple_benchmark_real_00512 ... bench:         998 ns/iter (+/- 115)     11543 MFLOPS
    test benchmark::simple_benchmark_real_01024 ... bench:       2,141 ns/iter (+/- 289)     11957 MFLOPS
    test benchmark::simple_benchmark_real_02048 ... bench:       4,448 ns/iter (+/- 1,232)   12661 MFLOPS
    test benchmark::simple_benchmark_real_04096 ... bench:       9,628 ns/iter (+/- 1,055)   12762 MFLOPS
    test benchmark::simple_benchmark_real_08192 ... bench:      20,542 ns/iter (+/- 3,241)   12960 MFLOPS
    test benchmark::simple_benchmark_real_16384 ... bench:      49,621 ns/iter (+/- 7,436)   11556 MFLOPS

### yFFT: Optimize generic real sequence kernel:

    test benchmark::simple_benchmark_real_00002 ... bench:           4 ns/iter (+/- 0)         1250 MFLOPS
    test benchmark::simple_benchmark_real_00004 ... bench:          20 ns/iter (+/- 13)        1000 MFLOPS
    test benchmark::simple_benchmark_real_00008 ... bench:          27 ns/iter (+/- 6)         2222 MFLOPS
    test benchmark::simple_benchmark_real_00016 ... bench:          48 ns/iter (+/- 26)        3333 MFLOPS
    test benchmark::simple_benchmark_real_00032 ... bench:          66 ns/iter (+/- 11)        6060 MFLOPS
    test benchmark::simple_benchmark_real_00064 ... bench:         125 ns/iter (+/- 26)        7680 MFLOPS
    test benchmark::simple_benchmark_real_00128 ... bench:         245 ns/iter (+/- 52)        9142 MFLOPS
    test benchmark::simple_benchmark_real_00256 ... bench:         522 ns/iter (+/- 486)       9808 MFLOPS
    test benchmark::simple_benchmark_real_00512 ... bench:       1,010 ns/iter (+/- 177)      11405 MFLOPS
    test benchmark::simple_benchmark_real_01024 ... bench:       2,173 ns/iter (+/- 418)      11780 MFLOPS
    test benchmark::simple_benchmark_real_02048 ... bench:       4,337 ns/iter (+/- 781)      12985 MFLOPS
    test benchmark::simple_benchmark_real_04096 ... bench:       9,349 ns/iter (+/- 1,917)    13143 MFLOPS
    test benchmark::simple_benchmark_real_08192 ... bench:      19,512 ns/iter (+/- 6,852)    13644 MFLOPS
    test benchmark::simple_benchmark_real_16384 ... bench:      45,774 ns/iter (+/- 13,310)   12527 MFLOPS
      
### yFFT: Add SSE3 real sequence kernel:

    test benchmark::simple_benchmark_real_00002 ... bench:           3 ns/iter (+/- 0)          1666 MFLOPS
    test benchmark::simple_benchmark_real_00004 ... bench:          21 ns/iter (+/- 3)           952 MFLOPS
    test benchmark::simple_benchmark_real_00008 ... bench:          26 ns/iter (+/- 15)         2307 MFLOPS
    test benchmark::simple_benchmark_real_00016 ... bench:          52 ns/iter (+/- 8)          3076 MFLOPS
    test benchmark::simple_benchmark_real_00032 ... bench:          61 ns/iter (+/- 17)         6557 MFLOPS
    test benchmark::simple_benchmark_real_00064 ... bench:         119 ns/iter (+/- 24)         8067 MFLOPS
    test benchmark::simple_benchmark_real_00128 ... bench:         214 ns/iter (+/- 144)       10467 MFLOPS
    test benchmark::simple_benchmark_real_00256 ... bench:         423 ns/iter (+/- 172)       12104 MFLOPS
    test benchmark::simple_benchmark_real_00512 ... bench:         871 ns/iter (+/- 463)       13226 MFLOPS
    test benchmark::simple_benchmark_real_01024 ... bench:       1,771 ns/iter (+/- 473)       14455 MFLOPS
    test benchmark::simple_benchmark_real_02048 ... bench:       3,925 ns/iter (+/- 744)       14349 MFLOPS
    test benchmark::simple_benchmark_real_04096 ... bench:       8,340 ns/iter (+/- 1,541)     14733 MFLOPS
    test benchmark::simple_benchmark_real_08192 ... bench:      18,589 ns/iter (+/- 13,140)    14322 MFLOPS
    test benchmark::simple_benchmark_real_16384 ... bench:      43,405 ns/iter (+/- 8,091)     13211 MFLOPS
           
### yFFT: Add AVX real sequence kernel (0675c51):

    test benchmark::simple_benchmark_real_00002 ... bench:           4 ns/iter (+/- 1)         1250 MFLOPS
    test benchmark::simple_benchmark_real_00004 ... bench:          22 ns/iter (+/- 3)          909 MFLOPS
    test benchmark::simple_benchmark_real_00008 ... bench:          28 ns/iter (+/- 6)         2142 MFLOPS
    test benchmark::simple_benchmark_real_00016 ... bench:          49 ns/iter (+/- 36)        3265 MFLOPS
    test benchmark::simple_benchmark_real_00032 ... bench:          54 ns/iter (+/- 12)        7407 MFLOPS
    test benchmark::simple_benchmark_real_00064 ... bench:         106 ns/iter (+/- 28)        9056 MFLOPS
    test benchmark::simple_benchmark_real_00128 ... bench:         175 ns/iter (+/- 31)       12800 MFLOPS
    test benchmark::simple_benchmark_real_00256 ... bench:         342 ns/iter (+/- 89)       14970 MFLOPS
    test benchmark::simple_benchmark_real_00512 ... bench:         741 ns/iter (+/- 118)      15546 MFLOPS
    test benchmark::simple_benchmark_real_01024 ... bench:       1,575 ns/iter (+/- 310)      16253 MFLOPS
    test benchmark::simple_benchmark_real_02048 ... bench:       3,201 ns/iter (+/- 637)      17594 MFLOPS
    test benchmark::simple_benchmark_real_04096 ... bench:       6,997 ns/iter (+/- 1,028)    17561 MFLOPS
    test benchmark::simple_benchmark_real_08192 ... bench:      16,426 ns/iter (+/- 2,998)    16208 MFLOPS
    test benchmark::simple_benchmark_real_16384 ... bench:      38,554 ns/iter (+/- 18,267)   14873 MFLOPS
    
### yFFT: Re-enable optimized bit reversal kernels (5d834ea)

    test benchmark::simple_benchmark_real_00002 ... bench:           3 ns/iter (+/- 1)           1666 MFLOPS
    test benchmark::simple_benchmark_real_00004 ... bench:          21 ns/iter (+/- 4)            952 MFLOPS
    test benchmark::simple_benchmark_real_00008 ... bench:          28 ns/iter (+/- 6)           2142 MFLOPS
    test benchmark::simple_benchmark_real_00016 ... bench:          47 ns/iter (+/- 8)           3404 MFLOPS
    test benchmark::simple_benchmark_real_00032 ... bench:          53 ns/iter (+/- 7)           7547 MFLOPS
    test benchmark::simple_benchmark_real_00064 ... bench:          89 ns/iter (+/- 17)         10786 MFLOPS
    test benchmark::simple_benchmark_real_00128 ... bench:         165 ns/iter (+/- 37)         13575 MFLOPS
    test benchmark::simple_benchmark_real_00256 ... bench:         346 ns/iter (+/- 61)         14797 MFLOPS
    test benchmark::simple_benchmark_real_00512 ... bench:         678 ns/iter (+/- 115)        16991 MFLOPS
    test benchmark::simple_benchmark_real_01024 ... bench:       1,514 ns/iter (+/- 235)        16908 MFLOPS
    test benchmark::simple_benchmark_real_02048 ... bench:       2,917 ns/iter (+/- 1,065)      19307 MFLOPS
    test benchmark::simple_benchmark_real_04096 ... bench:       6,655 ns/iter (+/- 938)        18464 MFLOPS
    test benchmark::simple_benchmark_real_08192 ... bench:      14,544 ns/iter (+/- 2,437)      18305 MFLOPS
    test benchmark::simple_benchmark_real_16384 ... bench:      34,699 ns/iter (+/- 7,888)      16526 MFLOPS
    
### yFFT: Replace all loads/stores with unaligned variant to accept inputs without 16-byte alignment
       
    test benchmark::simple_benchmark_00001      ... bench:           1 ns/iter (+/- 0)
    test benchmark::simple_benchmark_00002      ... bench:          13 ns/iter (+/- 2)
    test benchmark::simple_benchmark_00004      ... bench:          12 ns/iter (+/- 3)
    test benchmark::simple_benchmark_00008      ... bench:          25 ns/iter (+/- 3)
    test benchmark::simple_benchmark_00016      ... bench:          39 ns/iter (+/- 6)
    test benchmark::simple_benchmark_00032      ... bench:          68 ns/iter (+/- 25)
    test benchmark::simple_benchmark_00064      ... bench:         117 ns/iter (+/- 53)
    test benchmark::simple_benchmark_00128      ... bench:         263 ns/iter (+/- 48)
    test benchmark::simple_benchmark_00256      ... bench:         529 ns/iter (+/- 407)
    test benchmark::simple_benchmark_00512      ... bench:       1,115 ns/iter (+/- 162)
    test benchmark::simple_benchmark_01024      ... bench:       2,179 ns/iter (+/- 261)
    test benchmark::simple_benchmark_02048      ... bench:       5,179 ns/iter (+/- 595)
    test benchmark::simple_benchmark_04096      ... bench:      11,344 ns/iter (+/- 1,879)
    test benchmark::simple_benchmark_08192      ... bench:      29,249 ns/iter (+/- 4,906)
    test benchmark::simple_benchmark_16384      ... bench:      63,643 ns/iter (+/- 12,648)

### yFFT: Branch depending on the alignment on the input
       
    test benchmark::simple_benchmark_00001      ... bench:           1 ns/iter (+/- 0)
    test benchmark::simple_benchmark_00002      ... bench:          13 ns/iter (+/- 2)
    test benchmark::simple_benchmark_00004      ... bench:          13 ns/iter (+/- 3)
    test benchmark::simple_benchmark_00008      ... bench:          24 ns/iter (+/- 4)
    test benchmark::simple_benchmark_00016      ... bench:          36 ns/iter (+/- 29)
    test benchmark::simple_benchmark_00032      ... bench:          65 ns/iter (+/- 10)
    test benchmark::simple_benchmark_00064      ... bench:         112 ns/iter (+/- 22)
    test benchmark::simple_benchmark_00128      ... bench:         259 ns/iter (+/- 44)
    test benchmark::simple_benchmark_00256      ... bench:         497 ns/iter (+/- 72)
    test benchmark::simple_benchmark_00512      ... bench:       1,076 ns/iter (+/- 188)
    test benchmark::simple_benchmark_01024      ... bench:       2,265 ns/iter (+/- 2,450)
    test benchmark::simple_benchmark_02048      ... bench:       5,162 ns/iter (+/- 1,033)
    test benchmark::simple_benchmark_04096      ... bench:      11,558 ns/iter (+/- 3,204)
    test benchmark::simple_benchmark_08192      ... bench:      29,035 ns/iter (+/- 4,448)
    test benchmark::simple_benchmark_16384      ... bench:      63,373 ns/iter (+/- 13,011)

### Optimize complex multiplications

    test benchmark::simple_benchmark_00001      ... bench:           1 ns/iter (+/- 0)
    test benchmark::simple_benchmark_00002      ... bench:          12 ns/iter (+/- 8)            833 MFLOPS
    test benchmark::simple_benchmark_00004      ... bench:          11 ns/iter (+/- 7)           3636 MFLOPS
    test benchmark::simple_benchmark_00008      ... bench:          22 ns/iter (+/- 13)          5454 MFLOPS
    test benchmark::simple_benchmark_00016      ... bench:          34 ns/iter (+/- 23)          9411 MFLOPS
    test benchmark::simple_benchmark_00032      ... bench:          58 ns/iter (+/- 25)         13793 MFLOPS
    test benchmark::simple_benchmark_00064      ... bench:         101 ns/iter (+/- 41)         19009 MFLOPS
    test benchmark::simple_benchmark_00128      ... bench:         229 ns/iter (+/- 97)         19563 MFLOPS
    test benchmark::simple_benchmark_00256      ... bench:         447 ns/iter (+/- 311)        22908 MFLOPS
    test benchmark::simple_benchmark_00512      ... bench:       1,055 ns/iter (+/- 163)        21838 MFLOPS
    test benchmark::simple_benchmark_01024      ... bench:       2,035 ns/iter (+/- 355)        25159 MFLOPS
    test benchmark::simple_benchmark_02048      ... bench:       4,528 ns/iter (+/- 1,729)      24876 MFLOPS
    test benchmark::simple_benchmark_04096      ... bench:      10,247 ns/iter (+/- 6,245)      23983 MFLOPS
    test benchmark::simple_benchmark_08192      ... bench:      27,452 ns/iter (+/- 22,139)     19396 MFLOPS
    test benchmark::simple_benchmark_16384      ... bench:      57,073 ns/iter (+/- 32,198)     20094 MFLOPS

    test benchmark::simple_benchmark_real_00002 ... bench:           3 ns/iter (+/- 2)           1666 MFLOPS
    test benchmark::simple_benchmark_real_00004 ... bench:          19 ns/iter (+/- 10)          1052 MFLOPS
    test benchmark::simple_benchmark_real_00008 ... bench:          25 ns/iter (+/- 5)           2400 MFLOPS
    test benchmark::simple_benchmark_real_00016 ... bench:          43 ns/iter (+/- 15)          3720 MFLOPS
    test benchmark::simple_benchmark_real_00032 ... bench:          46 ns/iter (+/- 29)          8695 MFLOPS
    test benchmark::simple_benchmark_real_00064 ... bench:          80 ns/iter (+/- 47)         12000 MFLOPS
    test benchmark::simple_benchmark_real_00128 ... bench:         145 ns/iter (+/- 93)         15448 MFLOPS
    test benchmark::simple_benchmark_real_00256 ... bench:         309 ns/iter (+/- 126)        16569 MFLOPS
    test benchmark::simple_benchmark_real_00512 ... bench:         602 ns/iter (+/- 297)        19136 MFLOPS
    test benchmark::simple_benchmark_real_01024 ... bench:       1,284 ns/iter (+/- 427)        19937 MFLOPS
    test benchmark::simple_benchmark_real_02048 ... bench:       2,631 ns/iter (+/- 1,406)      21406 MFLOPS
    test benchmark::simple_benchmark_real_04096 ... bench:       5,911 ns/iter (+/- 4,057)      20788 MFLOPS
    test benchmark::simple_benchmark_real_08192 ... bench:      12,791 ns/iter (+/- 2,463)      20814 MFLOPS
    test benchmark::simple_benchmark_real_16384 ... bench:      33,221 ns/iter (+/- 28,459)     17261 MFLOPS
    
### Without bit reversal **NOT BENCHFFT COMPLIANT**

    test benchmark::simple_benchmark_00001      ... bench:           1 ns/iter (+/- 0)          
    test benchmark::simple_benchmark_00002      ... bench:           5 ns/iter (+/- 4)           2000 MFLOPS
    test benchmark::simple_benchmark_00004      ... bench:           5 ns/iter (+/- 3)           8000 MFLOPS
    test benchmark::simple_benchmark_00008      ... bench:          15 ns/iter (+/- 8)           8000 MFLOPS
    test benchmark::simple_benchmark_00016      ... bench:          21 ns/iter (+/- 3)          15238 MFLOPS
    test benchmark::simple_benchmark_00032      ... bench:          41 ns/iter (+/- 25)         19512 MFLOPS
    test benchmark::simple_benchmark_00064      ... bench:          68 ns/iter (+/- 13)         28235 MFLOPS
    test benchmark::simple_benchmark_00128      ... bench:         163 ns/iter (+/- 65)         27484 MFLOPS
    test benchmark::simple_benchmark_00256      ... bench:         311 ns/iter (+/- 85)         32926 MFLOPS
    test benchmark::simple_benchmark_00512      ... bench:         743 ns/iter (+/- 170)        31009 MFLOPS
    test benchmark::simple_benchmark_01024      ... bench:       1,519 ns/iter (+/- 341)        33706 MFLOPS
    test benchmark::simple_benchmark_02048      ... bench:       3,265 ns/iter (+/- 1,435)      34499 MFLOPS
    test benchmark::simple_benchmark_04096      ... bench:       7,334 ns/iter (+/- 1,190)      33509 MFLOPS
    test benchmark::simple_benchmark_08192      ... bench:      18,953 ns/iter (+/- 3,138)      28094 MFLOPS
    test benchmark::simple_benchmark_16384      ... bench:      35,599 ns/iter (+/- 6,333)      32216 MFLOPS


### yFFT: Optimize complex multiplications

    test benchmark::simple_benchmark_00001      ... bench:           1 ns/iter (+/- 0)
    test benchmark::simple_benchmark_00002      ... bench:           5 ns/iter (+/- 0)           2000 MFLOPS
    test benchmark::simple_benchmark_00004      ... bench:           5 ns/iter (+/- 3)           8000 MFLOPS

    test benchmark::simple_benchmark_real_00002 ... bench:           3 ns/iter (+/- 2)           1666 MFLOPS
    test benchmark::simple_benchmark_real_00004 ... bench:          17 ns/iter (+/- 3)           1176 MFLOPS
    test benchmark::simple_benchmark_real_00008 ... bench:          21 ns/iter (+/- 13)          2857 MFLOPS

### yFFT: Add AVX radix-2/4 bit reversal kernels

    test benchmark::simple_benchmark_00001      ... bench:           1 ns/iter (+/- 0)
    test benchmark::simple_benchmark_00002      ... bench:           5 ns/iter (+/- 0)               2000 MFLOPS
    test benchmark::simple_benchmark_00004      ... bench:           5 ns/iter (+/- 1)               8000 MFLOPS
    test benchmark::simple_benchmark_00008      ... bench:          22 ns/iter (+/- 10)              5454 MFLOPS
    test benchmark::simple_benchmark_00016      ... bench:          35 ns/iter (+/- 4)               9142 MFLOPS
    test benchmark::simple_benchmark_00032      ... bench:          65 ns/iter (+/- 11)             12307 MFLOPS
    test benchmark::simple_benchmark_00064      ... bench:          92 ns/iter (+/- 12)             20869 MFLOPS
    test benchmark::simple_benchmark_00128      ... bench:         240 ns/iter (+/- 145)            18666 MFLOPS
    test benchmark::simple_benchmark_00256      ... bench:         400 ns/iter (+/- 61)             25600 MFLOPS
    test benchmark::simple_benchmark_00512      ... bench:         974 ns/iter (+/- 271)            23655 MFLOPS
    test benchmark::simple_benchmark_01024      ... bench:       1,863 ns/iter (+/- 288)            27482 MFLOPS
    test benchmark::simple_benchmark_02048      ... bench:       4,629 ns/iter (+/- 2,248)          24333 MFLOPS
    test benchmark::simple_benchmark_04096      ... bench:      10,066 ns/iter (+/- 3,350)          24414 MFLOPS
    test benchmark::simple_benchmark_08192      ... bench:      25,325 ns/iter (+/- 4,255)          21025 MFLOPS
    test benchmark::simple_benchmark_16384      ... bench:      50,632 ns/iter (+/- 33,486)         22651 MFLOPS

    test benchmark::simple_benchmark_real_00002 ... bench:           4 ns/iter (+/- 0)               1250 MFLOPS
    test benchmark::simple_benchmark_real_00004 ... bench:          17 ns/iter (+/- 4)               1176 MFLOPS
    test benchmark::simple_benchmark_real_00008 ... bench:          23 ns/iter (+/- 2)               2608 MFLOPS
    test benchmark::simple_benchmark_real_00016 ... bench:          44 ns/iter (+/- 5)               3636 MFLOPS
    test benchmark::simple_benchmark_real_00032 ... bench:          48 ns/iter (+/- 7)               8333 MFLOPS
    test benchmark::simple_benchmark_real_00064 ... bench:          80 ns/iter (+/- 14)             12000 MFLOPS
    test benchmark::simple_benchmark_real_00128 ... bench:         136 ns/iter (+/- 18)             16470 MFLOPS
    test benchmark::simple_benchmark_real_00256 ... bench:         294 ns/iter (+/- 100)            17414 MFLOPS
    test benchmark::simple_benchmark_real_00512 ... bench:         546 ns/iter (+/- 80)             21098 MFLOPS
    test benchmark::simple_benchmark_real_01024 ... bench:       1,317 ns/iter (+/- 1,114)          19438 MFLOPS
    test benchmark::simple_benchmark_real_02048 ... bench:       2,540 ns/iter (+/- 447)            22173 MFLOPS
    test benchmark::simple_benchmark_real_04096 ... bench:       5,915 ns/iter (+/- 1,035)          20774 MFLOPS
    test benchmark::simple_benchmark_real_08192 ... bench:      12,634 ns/iter (+/- 1,554)          21073 MFLOPS
    test benchmark::simple_benchmark_real_16384 ... bench:      32,191 ns/iter (+/- 6,457)          17813 MFLOPS

### yFFT: Reverse the order of radixes

    test benchmark::simple_benchmark_00001      ... bench:           1 ns/iter (+/- 0)      
    test benchmark::simple_benchmark_00002      ... bench:           5 ns/iter (+/- 10)         2000 MFLOPS
    test benchmark::simple_benchmark_00004      ... bench:           5 ns/iter (+/- 1)          8000 MFLOPS
    test benchmark::simple_benchmark_00008      ... bench:          23 ns/iter (+/- 8)          5217 MFLOPS
    test benchmark::simple_benchmark_00016      ... bench:          30 ns/iter (+/- 12)        10666 MFLOPS
    test benchmark::simple_benchmark_00032      ... bench:          63 ns/iter (+/- 16)        12698 MFLOPS
    test benchmark::simple_benchmark_00064      ... bench:          91 ns/iter (+/- 14)        21098 MFLOPS
    test benchmark::simple_benchmark_00128      ... bench:         222 ns/iter (+/- 45)        20180 MFLOPS
    test benchmark::simple_benchmark_00256      ... bench:         399 ns/iter (+/- 81)        25664 MFLOPS
    test benchmark::simple_benchmark_00512      ... bench:         967 ns/iter (+/- 276)       23826 MFLOPS
    test benchmark::simple_benchmark_01024      ... bench:       1,753 ns/iter (+/- 938)       29207 MFLOPS
    test benchmark::simple_benchmark_02048      ... bench:       4,326 ns/iter (+/- 2,146)     26037 MFLOPS
    test benchmark::simple_benchmark_04096      ... bench:      10,174 ns/iter (+/- 3,979)     24155 MFLOPS
    test benchmark::simple_benchmark_08192      ... bench:      25,143 ns/iter (+/- 22,966)    21178 MFLOPS
    test benchmark::simple_benchmark_16384      ... bench:      48,723 ns/iter (+/- 30,472)    23538 MFLOPS

    test benchmark::simple_benchmark_real_00002 ... bench:           3 ns/iter (+/- 1)           1666 MFLOPS
    test benchmark::simple_benchmark_real_00004 ... bench:          16 ns/iter (+/- 10)          1250 MFLOPS
    test benchmark::simple_benchmark_real_00008 ... bench:          21 ns/iter (+/- 8)           2857 MFLOPS
    test benchmark::simple_benchmark_real_00016 ... bench:          39 ns/iter (+/- 8)           4102 MFLOPS
    test benchmark::simple_benchmark_real_00032 ... bench:          47 ns/iter (+/- 13)          8510 MFLOPS
    test benchmark::simple_benchmark_real_00064 ... bench:          87 ns/iter (+/- 16)         11034 MFLOPS
    test benchmark::simple_benchmark_real_00128 ... bench:         143 ns/iter (+/- 24)         15664 MFLOPS
    test benchmark::simple_benchmark_real_00256 ... bench:         299 ns/iter (+/- 99)         17123 MFLOPS
    test benchmark::simple_benchmark_real_00512 ... bench:         560 ns/iter (+/- 419)        20571 MFLOPS
    test benchmark::simple_benchmark_real_01024 ... bench:       1,284 ns/iter (+/- 223)        19937 MFLOPS
    test benchmark::simple_benchmark_real_02048 ... bench:       2,462 ns/iter (+/- 438)        22875 MFLOPS
    test benchmark::simple_benchmark_real_04096 ... bench:       5,936 ns/iter (+/- 1,082)      20700 MFLOPS
    test benchmark::simple_benchmark_real_08192 ... bench:      12,233 ns/iter (+/- 1,942)      21764 MFLOPS
    test benchmark::simple_benchmark_real_16384 ... bench:      28,135 ns/iter (+/- 5,381)      20381 MFLOPS

### yFFT: Disable AvxRadix4Kernel4 on large transforms

    test benchmark::simple_benchmark_00001      ... bench:           1 ns/iter (+/- 0)          
    test benchmark::simple_benchmark_00002      ... bench:           5 ns/iter (+/- 2)           2000 MFLOPS
    test benchmark::simple_benchmark_00004      ... bench:           5 ns/iter (+/- 2)           8000 MFLOPS
    test benchmark::simple_benchmark_00008      ... bench:          24 ns/iter (+/- 3)           5000 MFLOPS
    test benchmark::simple_benchmark_00016      ... bench:          28 ns/iter (+/- 1)          11428 MFLOPS
    test benchmark::simple_benchmark_00032      ... bench:          57 ns/iter (+/- 21)         14035 MFLOPS
    test benchmark::simple_benchmark_00064      ... bench:          89 ns/iter (+/- 13)         21573 MFLOPS
    test benchmark::simple_benchmark_00128      ... bench:         221 ns/iter (+/- 106)        20271 MFLOPS    
    test benchmark::simple_benchmark_00256      ... bench:         384 ns/iter (+/- 62)         26666 MFLOPS
    test benchmark::simple_benchmark_00512      ... bench:       1,001 ns/iter (+/- 82)         23016 MFLOPS
    test benchmark::simple_benchmark_01024      ... bench:       1,867 ns/iter (+/- 312)        27423 MFLOPS    
    test benchmark::simple_benchmark_02048      ... bench:       4,116 ns/iter (+/- 644)        27366 MFLOPS    
    test benchmark::simple_benchmark_04096      ... bench:      10,088 ns/iter (+/- 2,482)      24361 MFLOPS    
    test benchmark::simple_benchmark_08192      ... bench:      22,107 ns/iter (+/- 1,831)      24086 MFLOPS    
    test benchmark::simple_benchmark_16384      ... bench:      46,608 ns/iter (+/- 6,855)      24606 MFLOPS    

### yFFT: Add an improved benchmark program

    $ sudo nice -n -20 /Users/tcpp/Programs/Games/ngspades/NGSEngine/target/release/ysr2-benchmark
    Running benchmark...
    cplx-to-cplx, N =     1, t =      1.57, sd =     0.01,  mflops =      0.00
    cplx-to-cplx, N =     2, t =      5.88, sd =     0.04,  mflops =   1701.58
    cplx-to-cplx, N =     4, t =      5.82, sd =     0.07,  mflops =   6870.43
    cplx-to-cplx, N =     8, t =     25.27, sd =     0.22,  mflops =   4749.58
    cplx-to-cplx, N =    16, t =     31.70, sd =     0.15,  mflops =  10093.51
    cplx-to-cplx, N =    32, t =     66.75, sd =     0.56,  mflops =  11985.77
    cplx-to-cplx, N =    64, t =     92.76, sd =     1.53,  mflops =  20699.26
    cplx-to-cplx, N =   128, t =    227.94, sd =     1.80,  mflops =  19654.63
    cplx-to-cplx, N =   256, t =    407.29, sd =     2.71,  mflops =  25142.04
    cplx-to-cplx, N =   512, t =   1003.84, sd =    33.38,  mflops =  22951.89
    cplx-to-cplx, N =  1024, t =   1871.47, sd =    20.11,  mflops =  27358.24
    cplx-to-cplx, N =  2048, t =   4495.76, sd =    21.56,  mflops =  25054.69
    cplx-to-cplx, N =  4096, t =  10070.07, sd =    61.98,  mflops =  24404.99
    cplx-to-cplx, N =  8192, t =  24791.15, sd =   585.43,  mflops =  21478.63
    cplx-to-cplx, N = 16384, t =  50511.23, sd =   560.25,  mflops =  22705.45

### yFFT: Add AVX radix-2 kernel "AvxRadix2Kernel2"

The test execution time for every `benchmark_single` was set to 10 seconds.

    sudo nice -n -20 /Users/tcpp/Programs/Games/ngspades/NGSEngine/target/release/ysr2-benchmark
    Running benchmark...
    cplx-to-cplx, N =     1, t =      1.60, sd =     0.01,  mflops =      0.00
    cplx-to-cplx, N =     2, t =      5.89, sd =     0.02,  mflops =   1698.67
    cplx-to-cplx, N =     4, t =      5.98, sd =     0.04,  mflops =   6686.12
    cplx-to-cplx, N =     8, t =     24.40, sd =     0.15,  mflops =   4917.24
    cplx-to-cplx, N =    16, t =     35.50, sd =     0.18,  mflops =   9013.32
    cplx-to-cplx, N =    32, t =     61.18, sd =     0.32,  mflops =  13075.65
    cplx-to-cplx, N =    64, t =     96.79, sd =     0.40,  mflops =  19836.30
    cplx-to-cplx, N =   128, t =    197.32, sd =     1.31,  mflops =  22703.69
    cplx-to-cplx, N =   256, t =    411.46, sd =     1.86,  mflops =  24886.93
    cplx-to-cplx, N =   512, t =    899.80, sd =     6.70,  mflops =  25605.80
    cplx-to-cplx, N =  1024, t =   1865.69, sd =     6.06,  mflops =  27442.98
    cplx-to-cplx, N =  2048, t =   4107.98, sd =    32.07,  mflops =  27419.80
    cplx-to-cplx, N =  4096, t =  10273.73, sd =   170.51,  mflops =  23921.19
    cplx-to-cplx, N =  8192, t =  23220.60, sd =   327.64,  mflops =  22931.36
    cplx-to-cplx, N = 16384, t =  49975.73, sd =   414.43,  mflops =  22948.74

    test benchmark::simple_benchmark_real_00002 ... bench:           3 ns/iter (+/- 0)       1666 MFLOPS
    test benchmark::simple_benchmark_real_00004 ... bench:          16 ns/iter (+/- 9)       1250 MFLOPS
    test benchmark::simple_benchmark_real_00008 ... bench:          22 ns/iter (+/- 7)       2727 MFLOPS
    test benchmark::simple_benchmark_real_00016 ... bench:          38 ns/iter (+/- 18)      4210 MFLOPS
    test benchmark::simple_benchmark_real_00032 ... bench:          45 ns/iter (+/- 35)      8888 MFLOPS
    test benchmark::simple_benchmark_real_00064 ... bench:          72 ns/iter (+/- 30)     13333 MFLOPS
    test benchmark::simple_benchmark_real_00128 ... bench:         136 ns/iter (+/- 24)     16470 MFLOPS
    test benchmark::simple_benchmark_real_00256 ... bench:         277 ns/iter (+/- 97)     18483 MFLOPS
    test benchmark::simple_benchmark_real_00512 ... bench:         557 ns/iter (+/- 94)     20682 MFLOPS
    test benchmark::simple_benchmark_real_01024 ... bench:       1,136 ns/iter (+/- 186)    22535 MFLOPS
    test benchmark::simple_benchmark_real_02048 ... bench:       2,417 ns/iter (+/- 380)    23301 MFLOPS
    test benchmark::simple_benchmark_real_04096 ... bench:       5,224 ns/iter (+/- 1,960)  23522 MFLOPS
    test benchmark::simple_benchmark_real_08192 ... bench:      11,547 ns/iter (+/- 6,066)  23057 MFLOPS
    test benchmark::simple_benchmark_real_16384 ... bench:      27,201 ns/iter (+/- 8,086)  21081 MFLOPS

### yFFT: Add further optimizations

The test execution time for every `benchmark_single` was set to 10 seconds.

    cplx-to-cplx, N =     1, t =      1.57, sd =     0.01,  mflops =      0.00
    cplx-to-cplx, N =     2, t =      5.56, sd =     0.02,  mflops =   1797.38
    cplx-to-cplx, N =     4, t =      5.78, sd =     0.01,  mflops =   6916.35
    cplx-to-cplx, N =     8, t =     23.40, sd =     0.09,  mflops =   5128.30
    cplx-to-cplx, N =    16, t =     33.99, sd =     0.16,  mflops =   9414.20
    cplx-to-cplx, N =    32, t =     61.00, sd =     0.37,  mflops =  13115.31
    cplx-to-cplx, N =    64, t =     90.67, sd =     0.66,  mflops =  21175.67
    cplx-to-cplx, N =   128, t =    202.04, sd =     1.15,  mflops =  22173.66
    cplx-to-cplx, N =   256, t =    401.49, sd =     1.88,  mflops =  25504.96
    cplx-to-cplx, N =   512, t =    893.64, sd =    30.97,  mflops =  25782.30
    cplx-to-cplx, N =  1024, t =   1819.09, sd =    22.88,  mflops =  28145.95
    cplx-to-cplx, N =  2048, t =   4076.64, sd =    33.99,  mflops =  27630.60
    cplx-to-cplx, N =  4096, t =  10191.18, sd =    51.69,  mflops =  24114.96
    cplx-to-cplx, N =  8192, t =  23291.66, sd =   145.73,  mflops =  22861.41
    cplx-to-cplx, N = 16384, t =  49343.48, sd =   221.90,  mflops =  23242.79
