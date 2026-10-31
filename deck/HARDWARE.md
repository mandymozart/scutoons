# deck/HARDWARE.md – the machines and what deck-check measured

The classes of agents/research/CYBERDECKS.md §1 with the columns `deck-check` fills (deck/os/check; the report on tty1
at boot, `/var/lib/deck/machine.toml` on the stick). One row per machine; paste a new row from the report
(`deck-check` in a shell prints it again). The dogma numbers of deck/README.md (idle RAM, cold start) go here too
once measured on a deck.

| Machine | Class | CPU | RAM | GPU driver · Vulkan (driver, API) | VA-API decode / encode | Audio · PipeWire | MIDI / USB / serial | Features | Checked |
|---|---|---|---|---|---|---|---|---|---|
| QEMU VM (`npm run deck:vm --docker`, UEFI, virtio-vga, intel-hda, TCG – no KVM on the Windows host) | A | 2 × QEMU TCG CPU 2.5+ | 3900 MiB | virtio-pci · llvmpipe 1.4.354 (software, warned) | none / none | HDA Intel · 48 kHz / 1024 (PipeWire's own VM rule forces 1024; the deck config is read – allowed-rates applied) | – / QEMU USB keyboard, tablet, hard drive / – | display, audio; no video, encode, midi, editing | 2026-10-29: boot → Wi-Fi (wired) → check → cage + Chromium on the start page, the bridge joined the local relay (`{"sessions":1}`); the persistence partition after the MBR fix not re-verified (the owner stopped the slow TCG runs) |
| ThinkPad X1 Carbon Gen 7 (owner's; Whiskey Lake) | A | 8 × i7-8565U | 15664 MiB | i915 · Intel UHD Graphics 620 (WHL GT2), Intel open-source Mesa driver, 1.4.354 | h264 hevc jpeg mpeg2 vp8 vp9 / h264 hevc jpeg mpeg2 vp8 | none in the first ISO – sof-firmware missing (the SOF DSP + DMIC card needs it), added with the audio-degrades commit; PipeWire 48 kHz / 128 live | – / USB Disk 2.0, Chicony camera / – | display, video, encode, editing, audio | 2026-10-29 evening, the stage image (#179): deck-stage booted, "AV madness" at 60 fps (16.7 ms frame, 1.25 ms CPU per frame), audio over PipeWire 48 kHz, catpope 1's music online after ~8 s – the owner's reading of the F1 HUD. 2026-10-29 14:31Z, UEFI from the stick, Wi-Fi via nmtui (wlan0), wired enp0s31f6, wwan0; the kiosk did not start (the old must-have rule) – the first ISO's hotspot profile was rejected by NetworkManager (empty `interface-name=`), fixed in the same commit |
| Framework 13 AMD mainboard (the built deck) | A | | | | | | | | _not yet available_ |
| Rockchip RK3588 board | B | | | | | | | | _phase 3 (#174)_ |
| Raspberry Pi 5 | C | | | | | | | | _phase 3 (#174)_ |

How the class is decided (deck-check `classify`): x86_64 → **A**; ARM with hardware Vulkan and ≥ 6 GB → **B**; the rest
→ **C**. Features: `display` = any Vulkan (llvmpipe counts, with a warning), `video` = hardware Vulkan + a VA-API
decode profile (never on C), `video_encode` = an encode profile, `audio` = an ALSA card, `midi` = an ALSA sequencer
client, `editing` = A + hardware Vulkan + ≥ 8 GB. The one must-have (exit 1, no kiosk): a display device (a DRM driver);
everything else is a warning and a feature off (§1, "hijack any hardware around").

## Roles – who runs a deck where (owner 2026-10-31)
What a deck needs follows what it does in a show: a live-video deck is big, a choir singer's is small. We will build
hardware one day, so the numbers stay as low as each role allows. **First guesses, to be corrected by the log below** –
every test or show adds a row there, and the table follows what we learn.

| Role | Runs on (today → own hardware later) | Does | Storage (system + cache) | Compute | Must have |
|---|---|---|---|---|---|
| Live video / outlet | laptop (X1) → class A box | plays the show's cells: 1080p+ video, slices, the 3D set (#186) | 6 GB + **the show's videos: 20–100+ GB** | hardware video decode, a real GPU | HDMI out, wired network |
| Conductor | laptop / the PC | the session, every deck's being, pushes syncs (#195), cues | 6 GB + sets + portraits of all decks: ~10–20 GB | GPU for many beings (5 decks = 1.5 ms CPU / frame today) | screen, keyboard |
| AV deck (a being: 3D + synths) | laptop (X1) → class A / B | the cathedral being, its sound, a performer's knobs | 6 GB + sets + portraits: ~10 GB | GPU for 3D (UHD 620 holds 60 fps), audio | screen, audio out |
| Musician / synth deck | a small box → class B | sound only (deck-core), MIDI in / out, the tempo clock | ~8 GB | CPU for audio (16 FM voices = 10 µs) | audio interface, MIDI / USB |
| Light / device bridge | a tiny board → class C | DMX (USB Pro, Art-Net), OSC, MIDI to the hall's devices (#187) | ~4–8 GB | almost none | USB, network |
| Choir singer / performer | **the singer's phone** (browser, /live/deck) → maybe a small clip-on device | its voice part, cues, haptics, its being on the conductor (#190) | nothing installed; a browser cache of a few MB | a phone (WebGL2) | headphones, the session's QR code |
| Audience | their phone (browser) | votes, the show's view | nothing | a phone | the session link |

**Tiers, like a phone's storage** (owner: "like phones and their disks"): one deck image, sized by the role.

| Tier | Storage | RAM | For |
|---|---|---|---|
| S | 8 GB | 2–4 GB | light / device bridge, musician without samples |
| M | 32 GB | 8 GB | AV deck, musician with samples, conductor of a small session |
| L | 128 GB | 16 GB | conductor of a show, live video for one show |
| XL | 512 GB+ | 16–32 GB | live video across shows / a touring outlet |

The X1 install (#200) is an M today (20 GB); a phone in the browser needs no tier.

### Log – use cases and what we learned
| Date | Who / where | Roles used | Machines | What we learned |
|---|---|---|---|---|
| 2026-10-30/31 | owner, at home (3-device test, session CATH) | conductor · AV deck · phone deck | PC (RTX 2070), X1 on stick s5, S22 (Chrome) | X1 runs the cathedral at 60 fps; the stick's first boot hung on mkfs discard (fixed s6); Wi-Fi had to be retyped (fixed s6); typing the relay address is annoying (→ #199); the S22 has no WebGPU (→ WebGL2, #198); a phone reload left ghost decks (relay keep-alive); 20 GB plan for the X1 install, video makes the outlet role big (#200) |
