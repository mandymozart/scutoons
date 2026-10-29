# deck/os – the deck live image (class A x86)

The live USB stick of agents/research/CYBERDECKS.md §3 (#173): boot any x86 machine from it without touching its disk,
pick the Wi-Fi, `deck-check` measures the hardware, the suite starts in a kiosk. Install to a partition is optional.
Phase 2 of the deck plan; the binaries come from the workspace next door (deck/README.md).

## Build the ISO

Docker Desktop (or any Docker) is all the host needs – the build runs in an `archlinux:latest` container with
`--privileged` (mkarchiso's pacstrap chroots):

```bash
npm run deck:iso
```

→ `deck/os/out/deck-<date>-x86_64.iso` (+ `mkarchiso.log`). Two named volumes keep the cargo target and the pacman
cache between builds (`scutoons-deck-work`, `scutoons-deck-pkg`; `docker volume rm` both for a cold build). The first
run downloads ~1 GB of packages. `DECK_KERNEL=linux` builds on the stock kernel when `linux-rt` is not in the repos.

What `container/build.sh` does: installs archiso + rust → `cargo build --release` for `relay`, `bridge`, `deck-check`
→ copies archiso's **releng** profile, removes its live-only bits (root autologin, reflector, the mirror chooser,
networkd), overlays `profile/` (packages, pacman.conf, identity, airootfs) and `rootfs/` (the deck's units, scripts
and config – the same tree the PKGBUILD installs), drops the binaries in as `deck-relay`, `deck-bridge`,
`deck-check`, renames the kernel in the boot entries to `linux-rt`, adds a serial console → `mkarchiso`.

## Write it to a stick

Any raw write (balenaEtcher, Rufus in DD mode, `dd bs=4M`). The stick needs free space after the image: the first boot
creates the persistence partition `DECKDATA` there (`deck-persist`: sgdisk appends it, ext4, mounted at
`/var/lib/deck`; the pacman cache lives on it too). A stick that is exactly the ISO's size runs without persistence
and says so.

## Boot in a VM first

```bash
npm run deck:vm            # UEFI, a QEMU window; `-- bios` for SeaBIOS
npm run deck:vm -- --docker   # no QEMU on the host: QEMU inside a container, screenshots in deck/os/out/vm/
```

`vm/run.sh` copies the ISO to `out/vm/stick.img` with 4 GiB free (so persistence can be seen on the second boot) and
boots it as a USB stick with audio (intel-hda), a wired NIC, USB keyboard + tablet, 4 GB RAM. The serial console
(`ttyS0`) is the terminal: a root shell for `journalctl -u deck-firstboot -u deck-kiosk`, `deck-check`, `deck-install`.
Host QEMU on Windows: `winget install SoftwareFreedomConservancy.QEMU`. VirtualBox works as well: a new Linux VM,
the ISO as an optical drive (then without persistence) or `VBoxManage convertfromraw out/vm/stick.img stick.vdi` as
a disk with EFI enabled.

## What happens at boot (tty1)

1. `deck-firstboot.service` – persistence (`deck-persist`), the defaults into `/var/lib/deck` (`venue.toml`,
   `deck.toml`), the local relay starts.
2. `deck-wifi` – online already (wired, a remembered Wi-Fi) → 8 s countdown; else **W** nmtui's picker, **H** the deck's
   own hotspot (`deck-hotspot.nmconnection`: SSID `scutoons-deck`, key `scutoons-deck`, the deck is 10.42.0.1),
   **N** no network, **S** a shell.
3. `deck-check` – the report on screen, `/var/lib/deck/machine.toml` (class + features; CYBERDECKS.md §1). Once per
   stick; `deck.recheck` on the kernel line or `deck-check` in a shell re-runs it. A missing must-have (no Vulkan at
   all, no audio device) leaves a root shell instead of the kiosk.
4. `deck-kiosk.service` – `cage` + Chromium (`deck-session`) as the user `deck` on the page `conductor` names in
   `/var/lib/deck/deck.toml`; empty = the deck's own start page (the machine report, the relay's `/status`, the link
   to the conductor). `deck-bridge.service` joins the local relay with `/var/lib/deck/venue.toml`.

tty2 is a shell (Ctrl+Alt+F2), so is the serial port; ssh is key-only (no passwords on the image). The user `deck`
(uid 1000) is in audio, video, input, realtime, uucp; PipeWire runs at 48 kHz / quantum 128 with rtkit
(`rootfs/etc/pipewire/pipewire.conf.d/10-deck-rt.conf`) under `user@1000` (linger) so the check can read it.

## Install to a partition

`sudo deck-install` on the stick (a shell on tty2 or the serial console). It lists the machine's partitions (the
stick excluded), the user picks one by number and confirms by typing its name, optionally picks an EFI system
partition; then mkfs.ext4 → unsquashfs the live root → the live-only bits removed → fstab → mkinitcpio → a
systemd-boot entry on the ESP. It never creates or resizes partitions: make the partition first (cfdisk). The
installed deck boots the same flow (the check runs once, then the kiosk). Not yet tested on real hardware.

## Layout

```
deck/os/
  build.sh             the host wrapper (npm run deck:iso) – Docker, volumes, paths
  container/build.sh   inside the container: packages → cargo → releng + overlays → mkarchiso
  profile/             what differs from releng: packages.x86_64, pacman.conf, profiledef.deck.sh, airootfs/ (live only)
  rootfs/              the deck as installed on any system (ISO and PKGBUILD): usr/bin/deck-*, systemd units,
                       sysusers / tmpfiles, PipeWire + NetworkManager config, usr/share/deck (defaults, start page)
  check/               deck-check (Rust, a workspace member)
  vm/run.sh            QEMU as a USB stick (npm run deck:vm)
  out/                 the ISOs, logs, the VM stick image (git-ignored)
deck/pkg/scutoons-deck/PKGBUILD   the suite as one package (binaries + rootfs) for the deck's own repository
deck/HARDWARE.md                  the matrix of §1 with what deck-check measures, per machine
```

## Open (#173)

Pairing by QR / code with the API's device id · the pacman repository (`pkg.scutoons.com`, the PKGBUILD built by
CI) · the ARM images (phase 3, #174) · `deck-install` on real hardware · the ThinkPad X1 row in HARDWARE.md (owner).
