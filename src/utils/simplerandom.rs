// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::sync::OnceLock;
use std::time::Instant;

fn cpu_start_nanos() -> Option<u64> {
    // --- LINUX FALL ---
    #[cfg(target_os = "linux")]
    {
        use std::mem::MaybeUninit;
        // libc Definitionen sind in std::os::linux enthalten
        unsafe extern "C" {
            fn clock_gettime(clock_id: i32, tp: *mut Timespec) -> i32;
        }
        #[repr(C)]
        struct Timespec {
            tv_sec: i64,
            tv_nsec: i64,
        }
        const CLOCK_MONOTONIC: i32 = 1;

        let mut tp = MaybeUninit::<Timespec>::uninit();
        unsafe {
            if clock_gettime(CLOCK_MONOTONIC, tp.as_mut_ptr()) == 0 {
                let tp = tp.assume_init();
                return Some((tp.tv_sec as u64) * 1_000_000_000 + (tp.tv_nsec as u64));
            }
        }
    }

    // --- WINDOWS FALL ---
    #[cfg(target_os = "windows")]
    {
        unsafe extern "system" {
            fn QueryPerformanceCounter(lpPerformanceCount: *mut i64) -> i32;
            fn QueryPerformanceFrequency(lpFrequency: *mut i64) -> i32;
        }
        let mut counter = 0i64;
        let mut frequency = 0i64;
        unsafe {
            if QueryPerformanceCounter(&mut counter) != 0 && QueryPerformanceFrequency(&mut frequency) != 0 && frequency > 0 {
                // Umrechnung in Nanosekunden ohne Überlauf
                return Some(((counter as u128 * 1_000_000_000) / frequency as u128) as u64);
            }
        }
    }

    // Falls das Betriebssystem weder Linux noch Windows ist (oder der Aufruf fehlschlägt)
    None
}

fn monotonic_seed() -> u32 {
    static PROGRAM_START: OnceLock<Instant> = OnceLock::new();
    let prog_start = PROGRAM_START.get_or_init(Instant::now);

    let nanos = match cpu_start_nanos() {
        Some(uptime) => uptime,
        None => prog_start.elapsed().as_nanos() as u64,
    };

    // 1. Holen der Prozess-ID (Plattformübergreifend via std)
    let pid = std::process::id() as u64;

    // 2. Holen einer zufälligen Speicheradresse (nutzt ASLR des OS)
    let local_var = 0u8;
    let addr = (&local_var as *const u8 as usize) as u64;

    // 3. Kombinieren der Werte mittels XOR und Bit-Shifts
    let combined = nanos ^ (pid << 17) ^ (addr << 31);

    // Oberen und unteren 32-Bit-Teil miteinander verheiraten
    (combined ^ (combined >> 32)) as u32
}

pub struct SimpleRng {
    state: u32,
}

impl SimpleRng {
    pub fn new(seed: u32) -> Self {
        Self { state: if seed == 0 { 0x12345678 } else { seed } }
    }

    pub fn new_monotonic() -> Self {
        Self::new(monotonic_seed())
    }

    pub fn next_u32(&mut self) -> u32 {
        self.state = self.state
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);
        self.state
    }
}

