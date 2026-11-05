//! RT-safety evidence for the callback (ADR-008 D3): `Renderer::render`
//! must not allocate or free, in Rust or in the C++ stretch engine, while
//! key lock, SYNC, seeks, loops, tempo changes and track loads run.
//!
//! Rust: a counting global allocator armed only around `render` on this
//! thread. C++ (Signalsmith uses `new`/`std::vector`, which bypass Rust's
//! allocator): glibc's `mallinfo2` heap totals, read before and after every
//! `render`. Alone in this binary so no other test thread touches the heap.

use cpal as _;
use rtrb as _;
use signalsmith_stretch as _;

use oxidj_audio::{Command, Engine, TrackPcm};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;

struct Counting;

thread_local! {
    static ARMED: Cell<bool> = const { Cell::new(false) };
    static EVENTS: Cell<u64> = const { Cell::new(0) };
}

fn note() {
    let _ = ARMED.try_with(|armed| {
        if armed.get() {
            EVENTS.with(|e| e.set(e.get() + 1));
        }
    });
}

// SAFETY: forwards to the system allocator unchanged; only counts.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note();
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        note();
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        note();
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

#[cfg(all(target_os = "linux", target_env = "gnu"))]
mod heap {
    #[repr(C)]
    struct MallInfo2 {
        arena: usize,
        ordblks: usize,
        smblks: usize,
        hblks: usize,
        hblkhd: usize,
        usmblks: usize,
        fsmblks: usize,
        uordblks: usize,
        fordblks: usize,
        keepcost: usize,
    }
    unsafe extern "C" {
        fn mallinfo2() -> MallInfo2;
    }
    /// Bytes in use (heap + mmapped) and free-chunk count: any malloc or
    /// free in between changes at least one of them.
    pub fn in_use() -> Option<(usize, usize, usize)> {
        // SAFETY: plain glibc query, no arguments, returns by value.
        let m = unsafe { mallinfo2() };
        Some((m.uordblks, m.hblkhd, m.ordblks))
    }
}
#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
mod heap {
    pub fn in_use() -> Option<(usize, usize, usize)> {
        None
    }
}

fn track(rate: u32, secs: f64, hz: f64) -> Arc<TrackPcm> {
    let frames = (secs * f64::from(rate)) as usize;
    let samples = (0..frames)
        .flat_map(|i| {
            let t = i as f64 / f64::from(rate);
            let click = if (t * 2.0).fract() < 0.002 { 0.5 } else { 0.0 };
            let v = ((2.0 * std::f64::consts::PI * hz * t).sin() * 0.3 + click) as f32;
            [v, v]
        })
        .collect();
    Arc::new(TrackPcm::new(rate, samples).unwrap())
}

#[test]
fn render_never_allocates_with_key_lock_sync_and_transport() {
    let (mut engine, mut renderer) = Engine::manual(48_000);
    let tracks = [
        track(44_100, 30.0, 440.0),
        track(48_000, 30.0, 660.0),
        track(44_100, 20.0, 330.0),
    ];
    engine.send(Command::Load {
        deck: 0,
        id: 1,
        track: tracks[0].clone(),
    });
    engine.send(Command::Load {
        deck: 1,
        id: 2,
        track: tracks[1].clone(),
    });
    engine.send(Command::KeyLock { deck: 1, on: true });
    engine.send(Command::TogglePlay { deck: 0 });
    engine.send(Command::TogglePlay { deck: 1 });
    let mut out = vec![0.0f32; 512 * 2];
    let mut odd = vec![0.0f32; 333 * 2];
    let (mut rust_events, mut heap_changes, mut renders) = (0u64, 0u32, 0u32);
    for step in 0..1_500u32 {
        // UI side (unarmed): a different control every few blocks.
        let command = match step % 40 {
            3 => Some(Command::Speed {
                deck: 1,
                speed: 0.84 + f64::from(step % 7) * 0.05,
            }),
            7 => Some(Command::Seek {
                deck: 1,
                frame: f64::from(step % 13) * 30_000.0,
            }),
            11 => Some(Command::Sync {
                deck: 1,
                master: 0,
                speed: 1.05,
                beat: 24_000.0,
                anchor: 100.0,
                master_beat: 22_050.0,
                master_anchor: 0.0,
            }),
            15 => Some(Command::KeyLock {
                deck: 0,
                on: step % 80 == 15,
            }),
            19 => Some(Command::LoopIn { deck: 1 }),
            23 => Some(Command::LoopOut { deck: 1 }),
            25 => Some(Command::LoopScale {
                deck: 1,
                factor: 0.5,
            }),
            27 => Some(Command::LoopOut { deck: 1 }),
            29 => Some(Command::CueDown { deck: 0 }),
            30 => Some(Command::CueUp { deck: 0 }),
            31 => Some(Command::TogglePlay { deck: 0 }),
            35 => Some(Command::Load {
                deck: 0,
                id: u64::from(step),
                track: tracks[(step as usize / 40) % 3].clone(),
            }),
            36 => Some(Command::TogglePlay { deck: 0 }),
            _ => None,
        };
        if let Some(command) = command {
            engine.send(command);
        }
        let buf: &mut [f32] = if step % 5 == 0 { &mut odd } else { &mut out };
        // Audio side (armed): exactly what the device callback runs.
        let before = heap::in_use();
        ARMED.with(|a| a.set(true));
        renderer.render(buf, 2);
        ARMED.with(|a| a.set(false));
        let after = heap::in_use();
        rust_events += EVENTS.with(|e| e.replace(0));
        heap_changes += u32::from(before != after);
        renders += 1;
        engine.poll();
    }
    assert!(engine.deck(1).keylock && engine.deck(1).id == Some(2));
    println!("renders {renders}, Rust alloc/free {rust_events}, heap changes {heap_changes}");
    assert_eq!(rust_events, 0, "Rust allocations or frees inside render");
    assert_eq!(heap_changes, 0, "C/C++ heap changed inside render");

    // Control: both detectors see an allocation when one happens (building
    // a stretch engine allocates in Rust and in C++).
    let before = heap::in_use();
    ARMED.with(|a| a.set(true));
    let engine = oxidj_audio::Signalsmith::new(48_000);
    ARMED.with(|a| a.set(false));
    let after = heap::in_use();
    assert!(
        EVENTS.with(|e| e.replace(0)) > 0,
        "allocator counter is blind"
    );
    if before.is_some() {
        assert_ne!(before, after, "heap probe is blind");
    }
    drop(engine);
}
