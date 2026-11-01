# adapters-dmx-usbpro – USB-DMX for the bridge (#187)

DMX out over a USB widget that speaks the **Enttec DMX USB Pro** protocol (owner 2026-10-30, DECISIONS): the Enttec
DMX USB Pro itself and compatibles such as the DMXKing ultraDMX Micro / Pro / MAX. One device = one widget = one
universe (512 channels). The widget keeps the DMX line refreshed itself; the adapter only hands it a new universe.

## Venue patch

```toml
[[device]]
outlet = "lights"
adapter = "dmx-usbpro"
# port = "/dev/ttyUSB0"   # optional: "COM3" on Windows, "/dev/cu.usbserial-…" on macOS
```

Without `port` the first widget found is used: USB id 0403:6001 (FTDI, in the Enttec Pro and the older ultraDMX)
and USB names with DMX / Enttec / DMXking rank first (`src/detect.rs`). Two widgets = two devices with explicit ports.

## Messages

`device { outlet, msgs: [{ kind: "dmx", p: { ch, values, fade? } }] }` (protocol `DmxMessage`): `values` from channel
`ch` on (1-based, at most up to 512), set at once or faded linearly over `fade` ms from the current values. Several
ranges are several messages. A look (LIVE.md B.5.2: fixtures + looks) resolves to one message per fixture where the
show is – the bridge only plays channels. `panic` = blackout at once (fades dropped). The bridge's queue holds a
message until its `at`, as for every adapter.

## How it runs

- Its own thread owns the serial port (`serialport`, without libudev); it sleeps on its command channel until the
  next frame is due – no polling.
- Frames (label 6: start code 0 + 512 channels) only when something changed, at most every 25 ms (40 Hz), plus a
  keep-alive every second (the write also notices a pulled cable).
- `send_now` (adapters-core, #211 – a strobe's flash from deck-outputs): the change goes out in its own frame at once,
  at the earliest 4 ms after the last one, instead of on the 40 Hz grid. The widget puts the newest packet on the
  line with its next DMX frame (512 channels ≈ 23 ms per frame at 250 kbit/s).
- Device check on open: label 3 (parameters: firmware, break, MAB, rate) and label 10 (serial number), logged; a
  device that answers neither still gets frames (a warning names it).
- A widget missing at start or pulled out is looked for again every 2 s; the universe survives and is sent at once
  when it is back. Stopping the bridge leaves the last look standing (the widget repeats it) – a restart never
  flashes black; `panic` is the blackout.

## Linux (the deck)

FTDI widgets use the kernel's `ftdi_sio` → `/dev/ttyUSB*`; the CDC ones (ultraDMX MAX) `cdc_acm` → `/dev/ttyACM*`. Both
belong to the group `uucp` on Arch (`dialout` on Debian / Ubuntu) – the deck user is in it, no udev rule needed.

## Check the hardware

```bash
cd deck && cargo run -p adapters-dmx-usbpro --example dmx-probe -- 1=255,0,0 --hold 5
```

Lists the serial ports (`*` = a widget), opens one (`--port NAME` to pick), logs the device check, sets the values
(`CH=V,V,…`, `--fade MS`), holds them and blacks out. Mind a strobe's speed channel: start low.

## Tests

`cargo test -p adapters-dmx-usbpro` – against an in-memory widget (`src/mock.rs`): framing bytes, the device check,
frames on change at ≤ 40 Hz, the keep-alive, panic, a widget missing at start and pulled out, a silent clone,
detection. `bridge/tests/timing.rs` plays a DMX look through the real relay at its `at`.
