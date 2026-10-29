# deck/HARDWARE.md – the machines and what deck-check measured

The classes of agents/research/CYBERDECKS.md §1 with the columns `deck-check` fills (deck/os/check; the report on tty1
at boot, `/var/lib/deck/machine.toml` on the stick). One row per machine; paste a new row from the report
(`deck-check` in a shell prints it again). The dogma numbers of deck/README.md (idle RAM, cold start) go here too
once measured on a deck.

| Machine | Class | CPU | RAM | GPU driver · Vulkan (driver, API) | VA-API decode / encode | Audio · PipeWire | MIDI / USB / serial | Features | Checked |
|---|---|---|---|---|---|---|---|---|---|
| QEMU VM (`npm run deck:vm --docker`, UEFI, virtio-vga, intel-hda, TCG – no KVM on the Windows host) | A | 2 × QEMU TCG CPU 2.5+ | 3900 MiB | virtio-pci · llvmpipe 1.4.354 (software, warned) | none / none | HDA Intel · 48 kHz / 1024 (PipeWire's own VM rule forces 1024; the deck config is read – allowed-rates applied) | – / QEMU USB keyboard, tablet, hard drive / – | display, audio; no video, encode, midi, editing | 2026-10-29: boot → Wi-Fi (wired) → check → cage + Chromium on the start page, the bridge joined the local relay (`{"sessions":1}`); the persistence partition after the MBR fix not re-verified (the owner stopped the slow TCG runs) |
| ThinkPad X1 Carbon Gen 7 (owner's; Whiskey Lake) | A | 8 × i7-8565U | 15664 MiB | i915 · Intel UHD Graphics 620 (WHL GT2), Intel open-source Mesa driver, 1.4.354 | h264 hevc jpeg mpeg2 vp8 vp9 / h264 hevc jpeg mpeg2 vp8 | none in the first ISO – sof-firmware missing (the SOF DSP + DMIC card needs it), added with the audio-degrades commit; PipeWire 48 kHz / 128 live | – / USB Disk 2.0, Chicony camera / – | display, video, encode, editing; audio pending the fixed stick | 2026-10-29 14:31Z, UEFI from the stick, Wi-Fi via nmtui (wlan0), wired enp0s31f6, wwan0; the kiosk did not start (the old must-have rule) – the first ISO's hotspot profile was rejected by NetworkManager (empty `interface-name=`), fixed in the same commit |
| Framework 13 AMD mainboard (the built deck) | A | | | | | | | | _not yet available_ |
| Rockchip RK3588 board | B | | | | | | | | _phase 3 (#174)_ |
| Raspberry Pi 5 | C | | | | | | | | _phase 3 (#174)_ |

How the class is decided (deck-check `classify`): x86_64 → **A**; ARM with hardware Vulkan and ≥ 6 GB → **B**; the rest
→ **C**. Features: `display` = any Vulkan (llvmpipe counts, with a warning), `video` = hardware Vulkan + a VA-API
decode profile (never on C), `video_encode` = an encode profile, `audio` = an ALSA card, `midi` = an ALSA sequencer
client, `editing` = A + hardware Vulkan + ≥ 8 GB. The one must-have (exit 1, no kiosk): a display device (a DRM driver);
everything else is a warning and a feature off (§1, "hijack any hardware around").
