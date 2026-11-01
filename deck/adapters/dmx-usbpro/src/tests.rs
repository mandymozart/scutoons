//! The adapter against the in-memory widget (`mock`): device check, frames on change at ≤ 40 Hz, panic, a missing
//! and a pulled widget.

use std::time::Duration;

use protocol::{DeviceMsg, DmxMessage, MidiBytes};
use tokio::sync::mpsc;

use super::*;
use crate::mock::Widget;

fn dmx(ch: u64, values: &[u8], fade: Option<u64>) -> DeviceMsg {
    DeviceMsg::Dmx(DmxMessage { ch, values: values.to_vec(), fade })
}

fn started(widget: &Widget) -> DmxUsbProAdapter {
    let mut a = DmxUsbProAdapter::with_opener("lights", None, widget.opener());
    let (tx, _rx) = mpsc::channel(1);
    a.start(tx).unwrap();
    a
}

const SOON: Duration = Duration::from_secs(1);

#[test]
fn device_check_then_frames_on_change() {
    let widget = Widget::default();
    let mut a = started(&widget);
    assert!(widget.wait_frame(SOON, |f| f.iter().all(|&v| v == 0)).is_some(), "the universe goes out on connect");
    {
        let s = widget.0.lock().unwrap();
        assert_eq!(&s.labels[..3], &[3, 10, 6], "params, serial, then the first frame");
    }

    a.send(&dmx(1, &[255, 0, 128], None)).unwrap();
    assert!(widget.wait_frame(SOON, |f| f[..3] == [255, 0, 128]).is_some());

    // idle: no frame until the keep-alive
    let before = widget.frames().len();
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(widget.frames().len(), before, "nothing changed, no frame");

    // a burst: frames at most every 25 ms, the last one carries the last value – the bound follows the burst's real
    // length (a 2 ms sleep takes far longer on a busy CI runner: macOS stretched ~90 ms to ~450 ms)
    let start = std::time::Instant::now();
    for v in 0..=40u8 {
        a.send(&dmx(10, &[v], None)).unwrap();
        std::thread::sleep(Duration::from_millis(2));
    }
    let took = start.elapsed();
    assert!(widget.wait_frame(SOON, |f| f[9] == 40).is_some());
    let frames = widget.frames();
    let burst = &frames[before..];
    let most = (took.as_millis() / 25) as usize + 2;
    assert!(burst.len() <= most, "{took:?} of changes → ≤ {most} frames at 40 Hz, got {}", burst.len());
    for pair in burst.windows(2) {
        let gap = pair[1].0 - pair[0].0;
        assert!(gap >= Duration::from_millis(24), "frames {gap:?} apart");
    }
    a.stop().unwrap();
}

#[test]
fn keepalive_every_second() {
    let widget = Widget::default();
    let mut a = started(&widget);
    let first = widget.wait_frame(SOON, |_| true).unwrap();
    let n = widget.frames().len();
    std::thread::sleep(Duration::from_millis(1_150));
    let frames = widget.frames();
    assert_eq!(frames.len(), n + 1, "one keep-alive frame a second after the last");
    let gap = frames[n].0 - first;
    assert!(gap >= Duration::from_millis(990) && gap < Duration::from_millis(1_300), "keep-alive after {gap:?}");
    a.stop().unwrap();
}

#[test]
fn panic_blacks_out_at_once() {
    let widget = Widget::default();
    let mut a = started(&widget);
    a.send(&dmx(1, &[255; 8], None)).unwrap();
    a.send(&dmx(20, &[255; 4], Some(5_000))).unwrap();
    assert!(widget.wait_frame(SOON, |f| f[0] == 255).is_some());
    let t = std::time::Instant::now();
    a.panic().unwrap();
    let black = widget.wait_frame(SOON, |f| f.iter().all(|&v| v == 0)).expect("a black frame");
    assert!(black - t < Duration::from_millis(20), "black after {:?}", black - t);
    std::thread::sleep(Duration::from_millis(100));
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
    assert!(widget.wait_frame(RETRY + SOON, |f| f[4] == 77).is_some());
    assert_eq!(widget.0.lock().unwrap().opens, 1);

    // pulled: the next write fails; a value set meanwhile is kept; plugged again → one frame with both
    widget.set_unplugged(true);
    a.send(&dmx(6, &[88], None)).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    widget.set_unplugged(false);
    assert!(widget.wait_frame(RETRY + SOON, |f| f[4] == 77 && f[5] == 88).is_some());
    assert_eq!(widget.0.lock().unwrap().opens, 2);
    a.stop().unwrap();
}

#[test]
fn a_silent_clone_still_gets_frames() {
    let widget = Widget::default();
    widget.0.lock().unwrap().silent = true;
    let mut a = started(&widget);
    a.send(&dmx(1, &[42], None)).unwrap();
    assert!(widget.wait_frame(CHECK_TIMEOUT + SOON, |f| f[0] == 42).is_some());
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
    let mut a = started(&widget);
    assert!(widget.wait_frame(SOON, |_| true).is_some());
    a.send(&dmx(2, &[9], None)).unwrap();
    let plain = widget.wait_frame(SOON, |f| f[1] == 9).unwrap();
    let t = std::time::Instant::now();
    a.send_now(&dmx(1, &[255], None)).unwrap();
    let flash = widget.wait_frame(SOON, |f| f[0] == 255).expect("the flash");
    // measured bounds: about URGENT_GAP after the plain frame or later (the mock stamps a frame after its write, the
    // universe before it – a slow write of the plain frame shortens the stamped gap), and well before the 25 ms grid
    assert!(flash - plain >= universe::URGENT_GAP - Duration::from_millis(3), "{:?}", flash - plain);
    assert!(flash - t.max(plain + universe::URGENT_GAP) < Duration::from_millis(15), "the flash {:?} after send_now", flash - t);
    assert!(a.send_now(&DeviceMsg::Midi(MidiBytes { bytes: vec![0x90, 1, 1] })).is_err());
    a.stop().unwrap();
}
