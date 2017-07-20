//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate ngsgfx_common;
use ngsgfx_common::suballoc::*;
use std::ops;

struct Xorshift32(u32);

impl Xorshift32 {
    /// Returns a random integer in `[0, 0xfffffffe]`
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        !self.0
    }
    fn next_range(&mut self, range: ops::Range<u32>) -> u32 {
        let len = range.end - range.start;
        let mut mask = len - 1;
        mask |= mask >> 1;
        mask |= mask >> 2;
        mask |= mask >> 4;
        mask |= mask >> 8;
        mask |= mask >> 16;

        // Let's not care about the unbounded execution time :)
        let mut value = self.next() & mask;
        while value >= len {
            value = self.next() & mask;
        }

        value + range.start
    }
}

#[test]
fn create() {
    for i in 1..100 {
        println!("new({})", i);
        TlsfSuballoc::new(i as u32);
    }
}

#[test]
fn full_allocate() {
    for i in 1..100 {
        println!("new({})", i);
        let mut sa = TlsfSuballoc::new(i as u32);
        let result = sa.allocate(i as u32);
        assert!(result.is_some());
    }
}

#[test]
fn allocate_two() {
    for i in 1..50 {
        for k in 1..i {
            println!("new({})", i);
            let mut sa = TlsfSuballoc::new(i as u32);
            println!("  allocate({})", k);
            let result1 = sa.allocate(k as u32);
            assert!(result1.is_some());
            println!("  allocate({})", i - k);
            let result2 = sa.allocate((i - k) as u32);
            assert!(result2.is_some());
        }
    }
}

#[test]
fn allocate_three() {
    for i in 1..50 {
        for k in 1..i {
            for j in 1..i - k {
                println!("new({})", i);
                let mut sa = TlsfSuballoc::new(i as u32);
                println!("  allocate({})", k);
                let result1 = sa.allocate(k as u32);
                assert!(result1.is_some());
                println!("  allocate({})", i - k - j);
                let result2 = sa.allocate((i - k - j) as u32);
                assert!(result2.is_some());
                println!("  allocate({})", j);
                let result3 = sa.allocate((j) as u32);
                assert!(result3.is_some());
            }
        }
    }
}

#[test]
fn allocate_two_aligned() {
    for i in 1..50 {
        for k in 1..i - 8 {
            println!("new({})", i);
            let mut sa = TlsfSuballoc::new(i as u32);
            println!("  allocate({})", k);
            let result = sa.allocate(k as u32);
            assert!(result.is_some());
            println!("  allocate_aligned({}, 8)", i - k - 8);
            let result = sa.allocate_aligned((i - k - 8) as u32, 8);
            assert!(result.is_some());
            assert_eq!(result.as_ref().unwrap().1 & 7, 0, "unaligned: {:?}", result);
            println!("    success: {:?}", result);
        }
    }
}

#[test]
fn allocate_deallocate_two1() {
    for i in 1..50 {
        for k in 1..i {
            println!("new({})", i);
            let mut sa = TlsfSuballoc::new(i as u32);
            println!("  allocate({})", k);
            let result1 = sa.allocate(k as u32);
            assert!(result1.is_some());
            println!("  allocate({})", i - k);
            let result2 = sa.allocate((i - k) as u32);
            assert!(result2.is_some());

            println!("  deallocate(result1)");
            sa.deallocate(result1.unwrap().0);
            println!("  deallocate(result2)");
            sa.deallocate(result2.unwrap().0);
        }
    }
}

#[test]
fn stress() {
    let mut sa = TlsfSuballoc::new(1000u32);
    let mut allocated = Vec::new();
    let mut r = Xorshift32(0x11451419u32);
    for _ in 0..1000 {
        let len = 1u32 + (r.next() & 127u32);
        println!("allocate({})", len);
        let reg = sa.allocate(len);
        if let Some((reg, pos)) = reg {
            println!("  success: {:?}", (&reg, pos));
            allocated.push(reg);
        } else {
            assert!(allocated.len() > 0);
            let a_index = r.next_range(0..(allocated.len() as u32));
            let old_reg = allocated.swap_remove(a_index as usize);
            println!("  failed, deallocating {:?}", old_reg);
            sa.deallocate(old_reg);
        }
        if allocated.len() > 0 {
            sa.test_integrity(&allocated[0]);
        }
    }
    for reg in allocated {
        println!("deallocate({:?})", reg);
        sa.deallocate(reg);
    }

    // Try the full allocation
    println!("allocate({})", 1000u32);
    let reg = sa.allocate(1000u32);
    assert!(reg.is_some());
}

#[test]
fn stress_aligned() {
    let mut sa = TlsfSuballoc::new(4000u32);
    let mut allocated = Vec::new();
    let mut r = Xorshift32(0x11451419u32);
    for _ in 0..1000 {
        let len = 1u32 + (r.next() & 127u32);
        println!("allocate_aligned({}, {})", len, 64);
        let reg = sa.allocate_aligned(len, 64);
        if let Some((reg, pos)) = reg {
            println!("  success: {:?}", (&reg, pos));
            allocated.push(reg);
        } else {
            assert!(allocated.len() > 0);
            let a_index = r.next_range(0..(allocated.len() as u32));
            let old_reg = allocated.swap_remove(a_index as usize);
            println!("  failed, deallocating {:?}", old_reg);
            sa.deallocate(old_reg);
        }
        if allocated.len() > 0 {
            sa.test_integrity(&allocated[0]);
        }
    }
    for reg in allocated {
        println!("deallocate({:?})", reg);
        sa.deallocate(reg);
    }

    // Try the full allocation
    println!("allocate({})", 1000u32);
    let reg = sa.allocate(1000u32);
    assert!(reg.is_some());
}
