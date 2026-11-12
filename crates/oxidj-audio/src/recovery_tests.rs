//! Deterministic hardware-boundary injection. All candidate startup, callback,
//! poll/reopen, queue transfer and teardown code is the production implementation.
//! The parked worker owns a real Renderer but waits at a block boundary, keeping
//! pending commands deterministic without sleeps or a hardware device.
use super::*;
use output::{Callback, Candidate, DeviceStream, ErrorCallback, Output};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Counting;
thread_local! {
    static ARMED: Cell<bool> = const { Cell::new(false) };
    static EVENTS: Cell<usize> = const { Cell::new(0) };
}
fn note() {
    let _ = ARMED.try_with(|a| {
        if a.get() {
            EVENTS.with(|n| n.set(n.get() + 1));
        }
    });
}
// SAFETY: forwards unchanged to System; thread-local counters only observe.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Config,
    Build,
    Play,
    FatalStartup,
    Success,
}
#[derive(Default)]
struct Driver {
    callback: RefCell<Option<Callback>>,
    opens: Cell<usize>,
    builds: Cell<usize>,
    plays: Cell<usize>,
    drops: Cell<usize>,
}
struct FakeStream {
    driver: Rc<Driver>,
    error: ErrorCallback,
    stage: Stage,
}
impl DeviceStream for FakeStream {
    fn play(&self) -> Result<(), String> {
        self.driver.plays.set(self.driver.plays.get() + 1);
        let mut out = [1.0_f32; 32];
        self.driver
            .callback
            .borrow_mut()
            .as_mut()
            .unwrap()
            .render(&mut out, Duration::ZERO);
        assert_eq!(out, [0.0; 32], "play must not render speculative decks");
        if self.stage == Stage::FatalStartup {
            self.error.report(cpal::ErrorKind::DeviceNotAvailable);
        }
        if self.stage == Stage::Play {
            Err("injected play failure".into())
        } else {
            Ok(())
        }
    }
}
impl Drop for FakeStream {
    fn drop(&mut self) {
        self.driver.callback.borrow_mut().take();
        self.driver.drops.set(self.driver.drops.get() + 1);
    }
}
impl Driver {
    fn open(
        self: &Rc<Self>,
        stage: Stage,
        rate: u32,
        shared: Arc<Shared>,
    ) -> Result<Candidate, String> {
        self.opens.set(self.opens.get() + 1);
        if stage == Stage::Config {
            return Err("injected config failure".into());
        }
        Candidate::start(
            OutputKind::Device {
                name: "fake".into(),
                rate,
                channels: 4,
            },
            shared,
            |mut callback, error| {
                self.builds.set(self.builds.get() + 1);
                let mut out = [1.0_f32; 32];
                callback.render(&mut out, Duration::ZERO);
                assert_eq!(out, [0.0; 32], "build must not render speculative decks");
                error.report(cpal::ErrorKind::Xrun);
                if stage == Stage::Build {
                    return Err("injected build failure".into());
                }
                *self.callback.borrow_mut() = Some(callback);
                Ok(Box::new(FakeStream {
                    driver: self.clone(),
                    error,
                    stage,
                }))
            },
        )
    }

    fn render(&self, out: &mut [f32]) {
        EVENTS.with(|n| n.set(0));
        ARMED.with(|a| a.set(true));
        self.callback
            .borrow_mut()
            .as_mut()
            .unwrap()
            .render(out, Duration::ZERO);
        ARMED.with(|a| a.set(false));
        assert_eq!(
            EVENTS.with(Cell::get),
            0,
            "allocation/free in device callback (including first renderer transfer)"
        );
    }
}

fn parked() -> (Engine, Arc<AtomicBool>) {
    let (mut engine, mut renderer) = Engine::manual(48_000);
    let track = Arc::new(TrackPcm::new(48_000, vec![0.2; 200_000]).unwrap());
    for deck in 0..DECKS {
        assert!(engine.send(Command::Load {
            deck,
            id: 10 + deck as u64,
            track: track.clone()
        }));
    }
    for command in [
        Command::Seek {
            deck: 0,
            frame: 1000.0,
        },
        Command::CueDown { deck: 0 }, // set cue at 1000
        Command::LoopIn { deck: 0 },
        Command::Seek {
            deck: 0,
            frame: 9000.0,
        },
        Command::LoopOut { deck: 0 },
        Command::Seek {
            deck: 0,
            frame: 1000.0,
        },
        Command::CueDown { deck: 0 }, // preview, not ordinary play
        Command::Gain { deck: 0, gain: 0.5 },
        Command::Speed {
            deck: 0,
            speed: 1.25,
        },
        Command::KeyLock { deck: 1, on: true },
        Command::TogglePlay { deck: 1 },
        Command::Mixer(Some(MixerParams {
            master: 0.7,
            ..Default::default()
        })),
        Command::Limiter(false),
    ] {
        assert!(engine.send(command));
    }
    renderer.set_declick_ms(DECLICK_MS);
    renderer.set_limiter_lookahead_ms(LIMITER_LOOKAHEAD_MS);
    renderer.render(&mut [0.0; 512], 2);
    assert!(engine.deck(0).previewing);
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let thread = std::thread::spawn(move || {
        while !flag.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        renderer
    });
    engine.output = Output::Silent {
        stop: stop.clone(),
        thread: Some(thread),
    };
    engine.error = Some("missing device".into());
    engine.retry_at = Some(Instant::now());
    (engine, stop)
}

fn enqueue_transport(engine: &mut Engine) {
    // Not derivable from Memory or the last published snapshot. Order matters.
    for command in [
        Command::CueUp { deck: 0 },
        Command::LoopOut { deck: 0 }, // exit the already-active loop first
        Command::Seek {
            deck: 0,
            frame: 2000.0,
        },
        Command::LoopIn { deck: 0 },
        Command::Seek {
            deck: 0,
            frame: 6000.0,
        },
        Command::LoopOut { deck: 0 },
        Command::LoopScale {
            deck: 0,
            factor: 0.5,
        },
        Command::TogglePlay { deck: 0 },
    ] {
        assert!(engine.send(command));
    }
}

#[test]
fn recovery_config_build_play_and_startup_error_keep_silent_worker_and_queue() {
    for stage in [
        Stage::Config,
        Stage::Build,
        Stage::Play,
        Stage::FatalStartup,
    ] {
        let (mut engine, stop) = parked();
        let driver = Rc::new(Driver::default());
        let shared = engine.shared.clone();
        let before = engine.deck(0);
        enqueue_transport(&mut engine);
        let sent = engine.sent;
        engine.shared.stats.errors.store(7, Ordering::Relaxed);
        engine.poll_with(|_, shared| {
            assert!(
                !stop.load(Ordering::Acquire),
                "silent worker stopped before startup succeeded"
            );
            driver.open(stage, 96_000, shared)
        });
        assert!(
            !stop.load(Ordering::Acquire),
            "{stage:?} replaced/stopped the old silent worker"
        );
        assert!(Arc::ptr_eq(&engine.shared, &shared));
        assert_eq!(engine.sent, sent);
        assert_eq!(engine.deck(0).frame, before.frame);
        assert!(engine.deck(0).previewing);
        assert!(engine.pending(0));
        assert_eq!(engine.device_rate(), 48_000);
        assert_eq!(engine.stats().callbacks, 0);
        assert_eq!(
            engine.stats().xruns,
            0,
            "speculative error changed live counters"
        );
        assert_eq!(engine.stats().errors, 7);
        assert!(!shared.stats.dead.load(Ordering::Acquire));
        assert!(engine.retry_at.unwrap() > Instant::now());
        let expected = match stage {
            Stage::Config => "injected config failure",
            Stage::Build => "injected build failure",
            Stage::Play => "injected play failure",
            Stage::FatalStartup => "audio output failed during startup",
            Stage::Success => unreachable!(),
        };
        assert_eq!(engine.error(), Some(expected));
        // Drain the SAME queued consumer after failure and assert ordered effects.
        let mut renderer = engine.output.reclaim().unwrap();
        renderer.render(&mut [], 2);
        assert!(!engine.pending(0));
        let state = &renderer.decks[0];
        assert_eq!(
            (state.pos, state.cue, state.loop_in, state.loop_range),
            (2000.0, 1000.0, Some(2000.0), Some((2000.0, 4000.0)))
        );
        assert!(state.playing && !state.previewing);
        assert_eq!((state.gain, state.speed), (0.5, 1.25));
    }
}

#[test]
fn recovery_commits_the_started_candidate_once_and_returns_renderer_before_first_callback() {
    let (mut engine, stop) = parked();
    let driver = Rc::new(Driver::default());
    let shared = engine.shared.clone();
    let before = engine.deck(0);
    // A full ring is preserved even before the first callback can acquire it.
    for i in 0..COMMAND_CAPACITY {
        assert!(engine.send(Command::Seek {
            deck: 0,
            frame: 2000.0 + i as f64
        }));
    }
    let sent = engine.sent;
    engine.poll_with(|_, shared| driver.open(Stage::Success, 48_000, shared));
    assert!(stop.load(Ordering::Acquire));
    assert_eq!(
        (driver.opens.get(), driver.builds.get(), driver.plays.get()),
        (1, 1, 1)
    );
    assert_eq!(engine.device_rate(), 48_000);
    assert!(matches!(engine.output(), OutputKind::Device { .. }));
    assert!(engine.error().is_none() && engine.retry_at.is_none());
    assert!(Arc::ptr_eq(&engine.shared, &shared));
    assert_eq!(engine.sent, sent);
    assert!(engine.tx.is_full());
    engine.poll_with(|_, _| panic!("success must not reopen the device"));
    let mut renderer = engine
        .output
        .reclaim()
        .expect("callback destructor returns unconsumed handover");
    assert_eq!(driver.drops.get(), 1);
    assert!(Arc::ptr_eq(&renderer.shared, &shared));
    assert_eq!(renderer.decks[0].pos, before.frame);
    assert!(renderer.decks[0].previewing);
    assert!(
        renderer.decks[1].primed,
        "same-rate handover must preserve stretch history"
    );
    assert!(!renderer.limiter.on);
    assert_eq!(renderer.mixer.params.unwrap().master, 0.7);
    renderer.render(&mut [], 2);
    assert!(!engine.pending(0));
    assert_eq!(
        renderer.decks[0].pos,
        2000.0 + (COMMAND_CAPACITY - 1) as f64
    );
    assert!(engine.send(Command::CueUp { deck: 0 }));
    renderer.render(&mut [], 2);
    assert!(!renderer.decks[0].playing && !renderer.decks[0].previewing);
}

#[test]
fn recovery_rate_change_rebuilds_dsp_not_queue_and_first_callback_is_allocation_free() {
    for rate in [44_100, 88_200, 96_000] {
        let (mut engine, _) = parked();
        let driver = Rc::new(Driver::default());
        let shared = engine.shared.clone();
        let before = engine.deck(0);
        enqueue_transport(&mut engine);
        let sent = engine.sent;
        engine.poll_with(|_, shared| driver.open(Stage::Success, rate, shared));
        assert_eq!(engine.device_rate(), rate);
        let mut out = [0.0; 512 * 4];
        driver.render(&mut out);
        assert!(out.iter().all(|s| s.is_finite()));
        assert_eq!(engine.stats().callbacks, 1);
        assert_eq!(engine.sent, sent);
        assert!(!engine.pending(0));
        assert!(Arc::ptr_eq(&engine.shared, &shared));
        let mut renderer = engine.output.reclaim().unwrap();
        assert_eq!(renderer.device_rate(), rate);
        assert_eq!(renderer.decks[0].cue, before.cue);
        assert_eq!(renderer.decks[0].loop_range, Some((2000.0, 4000.0)));
        let expected = 2000.0 + 512.0 * 48_000.0 / f64::from(rate) * 1.25;
        assert!((renderer.decks[0].pos - expected).abs() < 1e-6);
        assert!(renderer.decks[1].primed && renderer.decks[1].keylock);
        assert_eq!(
            renderer.limiter.delay.len() / 2,
            (f64::from(rate) / 1000.0) as usize
        );
        assert!(!renderer.limiter.on);
        assert_eq!(renderer.mixer.params.unwrap().master, 0.7);
        // A second retarget while primed must reset old-rate read-ahead safely.
        let pos = renderer.decks[1].pos;
        renderer.retarget(48_000);
        assert!(!renderer.decks[1].primed);
        renderer.render(&mut [0.0; 256], 2);
        assert!((renderer.decks[1].pos - pos - 128.0).abs() < 1e-6);
    }
}

#[test]
fn explicit_reopen_closes_stream_before_open_and_preserves_pending_on_failure() {
    let (mut engine, _) = parked();
    let first = Rc::new(Driver::default());
    engine.poll_with(|_, shared| first.open(Stage::Success, 48_000, shared));
    enqueue_transport(&mut engine);
    let shared = engine.shared.clone();
    let sent = engine.sent;
    engine.rebuild_with(None, |_, _| {
        assert_eq!(
            first.drops.get(),
            1,
            "exclusive hardware must be closed first"
        );
        Err("exclusive reopen failed".into())
    });
    assert_eq!(engine.output(), OutputKind::Silent);
    assert!(Arc::ptr_eq(&engine.shared, &shared));
    assert_eq!(engine.sent, sent);
    let mut renderer = engine.output.reclaim().unwrap();
    renderer.render(&mut [], 2);
    assert!(!engine.pending(0));
    assert_eq!(renderer.decks[0].loop_range, Some((2000.0, 4000.0)));
}
