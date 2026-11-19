//! The adapter against the in-memory widget (`mock`): device check, frames on change at ≤ 40 Hz, panic, a missing
//! and a pulled widget. The timing tests run the adapter thread on a virtual clock (#639: wall-time budgets failed on
//! a loaded machine); the others only wait for a frame, with patience.

use std::sync::Arc;
use std::time::{Duration, Instant};

use protocol::{DeviceMsg, DmxMessage, MidiBytes};
use tokio::sync::mpsc;

use super::*;
use crate::clock::virtual_clock::Virtual;
use crate::mock::Widget;
use crate::universe::{FRAME_INTERVAL, KEEPALIVE, URGENT_GAP};

fn dmx(ch: u64, values: &[u8], fade: Option<u64>) -> DeviceMsg {
    DeviceMsg::Dmx(DmxMessage { ch, values: values.to_vec(), fade })
}

fn started(widget: &Widget) -> DmxUsbProAdapter {
    let mut a = DmxUsbProAdapter::with_opener("lights", None, widget.opener());
    let (tx, _rx) = mpsc::channel(1);
    a.start(tx).unwrap();
    a
}

/// For a frame that must come at all (not a timing bound): only a hang fails on it.
const PATIENCE: Duration = Duration::from_secs(60);

/// Started on a virtual clock that also stamps the widget's frames; settled on the first frame.
fn on_virtual_clock(widget: &Widget) -> (DmxUsbProAdapter, Arc<Virtual>) {
    let clock = Arc::new(Virtual::new());
    widget.0.lock().unwrap().clock = Some(clock.clone());
    let mut a = DmxUsbProAdapter::with_opener("lights", None, widget.opener()).with_clock(clock.clone());
    let (tx, _rx) = mpsc::channel(1);
    a.start(tx).unwrap();
    clock.settle();
    (a, clock)
}

/// The last frame and when it came.
fn last(widget: &Widget) -> (Instant, [u8; 512]) {
    *widget.0.lock().unwrap().frames.last().expect("a frame")
}

#[test]
fn device_check_then_frames_on_change() {
    let widget = Widget::default();
    let (mut a, clock) = on_virtual_clock(&widget);
    let (t0, f) = last(&widget);
    assert!(f.iter().all(|&v| v == 0), "the universe goes out on connect");
    assert_eq!(t0, clock.now(), "at once");
    {
        let s = widget.0.lock().unwrap();
        assert_eq!(&s.labels[..3], &[3, 10, 6], "params, serial, then the first frame");
    }

    a.send(&dmx(1, &[255, 0, 128], None)).unwrap();
    clock.advance(FRAME_INTERVAL);
    assert_eq!(last(&widget).1[..3], [255, 0, 128]);

    // idle: no frame until the keep-alive
    let before = widget.frames().len();
    clock.advance(Duration::from_millis(300));
    assert_eq!(widget.frames().len(), before, "nothing changed, no frame");

    // a burst every 2 ms: frames at most every 25 ms, the last one carries the last value
    let start = clock.now();
    for v in 0..=40u8 {
        a.send(&dmx(10, &[v], None)).unwrap();
        clock.advance(Duration::from_millis(2));
    }
    let took = clock.now() - start;
    clock.advance(FRAME_INTERVAL);
    assert_eq!(last(&widget).1[9], 40);
    let frames = widget.frames();
    let burst = &frames[before..];
    let most = (took.as_millis() / 25) as usize + 2;
    assert!(burst.len() <= most, "{took:?} of changes → ≤ {most} frames at 40 Hz, got {}", burst.len());
    for pair in burst.windows(2) {
        let gap = pair[1].0 - pair[0].0;
        assert!(gap >= FRAME_INTERVAL, "frames {gap:?} apart");
    }
    a.stop().unwrap();
}

#[test]
fn keepalive_every_second() {
    let widget = Widget::default();
    let (mut a, clock) = on_virtual_clock(&widget);
    let (first, _) = last(&widget);
    let n = widget.frames().len();
    clock.advance(Duration::from_millis(1_150));
    let frames = widget.frames();
    assert_eq!(frames.len(), n + 1, "one keep-alive frame a second after the last");
    assert_eq!(frames[n].0 - first, KEEPALIVE, "keep-alive after a second");
    a.stop().unwrap();
}

#[test]
fn panic_blacks_out_at_once() {
    let widget = Widget::default();
    let (mut a, clock) = on_virtual_clock(&widget);
    a.send(&dmx(1, &[255; 8], None)).unwrap();
    a.send(&dmx(20, &[255; 4], Some(5_000))).unwrap();
    clock.advance(FRAME_INTERVAL);
    assert_eq!(last(&widget).1[0], 255);
    clock.advance(Duration::from_millis(5));
    let t = clock.now();
    a.panic().unwrap();
    clock.settle();
    let (black, f) = last(&widget);
    assert_eq!(f, [0; 512], "a black frame");
    assert_eq!(black, t, "black at once, not on the 40 Hz grid");
    clock.advance(Duration::from_millis(100));
    assert_eq!(widget.last_frame().unwrap(), [0; 512], "the fade does not come back");
    a.stop().unwrap();
}

#[test]
fn missing_at_start_then_pulled_and_back() {
    let widget = Widget::default();
    widget.set_unplugged(true);
    let mut a = started(&widget);
    a.send(&dmx(5, &[77], None)).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert!(widget.frames().is_empty());

    // plugged in: found within the retry, the value set while it was missing goes out
    widget.set_unplugged(false);
    assert!(widget.wait_frame(PATIENCE, |f| f[4] == 77).is_some());
    assert_eq!(widget.0.lock().unwrap().opens, 1);

    // pulled: the next write fails; a value set meanwhile is kept; plugged again → one frame with both
    widget.set_unplugged(true);
    a.send(&dmx(6, &[88], None)).unwrap();
    // plugged again only once the adapter has seen the write fail (a fixed sleep raced it on a loaded machine, #639)
    let end = Instant::now() + PATIENCE;
    while widget.0.lock().unwrap().failed_writes == 0 {
        assert!(Instant::now() < end, "the write to the pulled widget never came");
        std::thread::sleep(Duration::from_millis(2));
    }
    widget.set_unplugged(false);
    assert!(widget.wait_frame(PATIENCE, |f| f[4] == 77 && f[5] == 88).is_some());
    assert_eq!(widget.0.lock().unwrap().opens, 2);
    a.stop().unwrap();
}

#[test]
fn a_silent_clone_still_gets_frames() {
    let widget = Widget::default();
    widget.0.lock().unwrap().silent = true;
    let mut a = started(&widget);
    a.send(&dmx(1, &[42], None)).unwrap();
    assert!(widget.wait_frame(PATIENCE, |f| f[0] == 42).is_some());
    a.stop().unwrap();
}

#[test]
fn the_port_opens_at_the_dmx_baud() {
    // the Eurolite USB-DMX512-PRO MK2 is silent at 115200 (#261)
    assert_eq!(BAUD, 250_000);
}

#[test]
fn check_reads_params_and_serial() {
    let widget = Widget::default();
    let (_, mut port) = widget.opener()().unwrap();
    let info = check_widget(port.as_mut(), CHECK_TIMEOUT).unwrap();
    assert_eq!(info.params.unwrap().firmware, 0x0144);
    assert_eq!(info.serial.as_deref(), Some("00123456"));
}

#[test]
fn refuses_what_it_cannot_play() {
    let widget = Widget::default();
    let mut idle = DmxUsbProAdapter::with_opener("lights", None, widget.opener());
    assert!(idle.send(&dmx(1, &[1], None)).is_err(), "not started");
    let mut a = started(&widget);
    assert!(a.send(&DeviceMsg::Midi(MidiBytes { bytes: vec![0x90, 60, 100] })).is_err());
    assert!(a.send(&dmx(0, &[1], None)).is_err());
    assert!(a.send(&dmx(512, &[1, 2], None)).is_err());
    assert!(a.send(&dmx(512, &[1], None)).is_ok());
    assert_eq!(a.name(), "dmx-usbpro");
    a.stop().unwrap();
    assert!(a.send(&dmx(1, &[1], None)).is_err(), "stopped");
}

#[test]
fn send_now_goes_out_in_its_own_frame() {
    // #211: a strobe's flash right after a frame must not wait for the 40 Hz grid
    let widget = Widget::default();
    let (mut a, clock) = on_virtual_clock(&widget);
    // 1 ms after a plain frame: URGENT_GAP after it; 10 ms after: at once – both well before the 25 ms grid
    for (v, after) in [(1u8, 1u64), (2, 10)] {
        a.send(&dmx(2, &[v], None)).unwrap();
        // up to the plain frame's grid moment, no further
        clock.advance(last(&widget).0 + FRAME_INTERVAL - clock.now());
        let (plain, f) = last(&widget);
        assert_eq!(f[1], v, "the plain frame");
        assert_eq!(plain, clock.now());
        clock.advance(Duration::from_millis(after));
        let t = clock.now();
        a.send_now(&dmx(1, &[255 - v], None)).unwrap();
        clock.advance(URGENT_GAP);
        let (flash, f) = last(&widget);
        assert_eq!(f[..2], [255 - v, v], "the flash");
        assert_eq!(flash, t.max(plain + URGENT_GAP), "the flash {after} ms after the plain frame came {:?} after it", flash - plain);
        assert!(flash < plain + FRAME_INTERVAL, "before the grid");
    }
    assert!(a.send_now(&DeviceMsg::Midi(MidiBytes { bytes: vec![0x90, 1, 1] })).is_err());
    a.stop().unwrap();
}
