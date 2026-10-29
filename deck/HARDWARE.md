# deck/HARDWARE.md – the machines and what deck-check measured

The classes of agents/research/CYBERDECKS.md §1 with the columns `deck-check` fills (deck/os/check; the report on tty1
at boot, `/var/lib/deck/machine.toml` on the stick). One row per machine; paste a new row from the report
(`deck-check` in a shell prints it again). The dogma numbers of deck/README.md (idle RAM, cold start) go here too
once measured on a deck.

| Machine | Class | CPU | RAM | GPU driver · Vulkan (driver, API) | VA-API decode / encode | Audio · PipeWire | MIDI / USB / serial | Features | Checked |
|---|---|---|---|---|---|---|---|---|---|
| QEMU VM (`npm run deck:vm`, virtio-vga, intel-hda, TCG) | A | 2 × QEMU Virtual CPU | 4 GB | virtio_gpu · llvmpipe (software) | none / none | HDA Intel (hda-duplex) · 48 kHz / 128 | – / QEMU keyboard, tablet / – | display, audio; no video, no editing | _pending: the first VM boot of the built ISO – see #173_ |
| ThinkPad X1 (owner's) | A | | | | | | | | _owner step: boot the stick, paste the report (#173)_ |
| Framework 13 AMD mainboard (the built deck) | A | | | | | | | | _not yet available_ |
| Rockchip RK3588 board | B | | | | | | | | _phase 3 (#174)_ |
| Raspberry Pi 5 | C | | | | | | | | _phase 3 (#174)_ |

How the class is decided (deck-check `classify`): x86_64 → **A**; ARM with hardware Vulkan and ≥ 6 GB → **B**; the rest
→ **C**. Features: `display` = any Vulkan (llvmpipe counts, with a warning), `video` = hardware Vulkan + a VA-API
decode profile (never on C), `video_encode` = an encode profile, `audio` = an ALSA card, `midi` = an ALSA sequencer
client, `editing` = A + hardware Vulkan + ≥ 8 GB. Must-haves (exit 1, no kiosk): a Vulkan device at all, an audio device.
