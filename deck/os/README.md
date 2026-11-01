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

→ `deck/os/out/scutoonsdeck-<date>-x86_64.iso` (volume label `SCUTOONSDECK`, hostname `scutoonsdeck`; + `mkarchiso.log`). Two named volumes keep the cargo target and the pacman
cache between builds (`scutoons-deck-work`, `scutoons-deck-pkg`; `docker volume rm` both for a cold build). The first
run downloads ~1 GB of packages. `DECK_KERNEL=linux` builds on the stock kernel when `linux-rt` is not in the repos.

What `container/build.sh` does: installs archiso + rust (+ pipewire, clang for the stage) → `cargo build --release` for
`relay`, `bridge`, `deck-check`, `deck-stage` (feature `pipewire`), `deck-link` → copies archiso's **releng** profile, removes its live-only bits (root autologin, reflector, the mirror chooser,
networkd), overlays `profile/` (packages, pacman.conf, identity, airootfs) and `rootfs/` (the deck's units, scripts
and config – the same tree the PKGBUILD installs), drops the binaries in as `deck-relay`, `deck-bridge`,
`deck-check`, `deck-stage`, `deck-link` (+ its scenes in `/usr/share/deck/scenes`), renders the logo (deck/stage/assets, the
official SVG in white) into plymouth's `logo.png` and the BIOS menu's `splash.png` (librsvg + imagemagick), renames the
kernel in the boot entries to `linux-rt`, adds a serial console → `mkarchiso`.

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
Without a terminal (an agent) the Docker mode logs the serial console to `out/vm/serial.log` and keeps it reachable
with `docker exec <vm container> socat - UNIX-CONNECT:/tmp/deck-serial`; screenshots land in `out/vm/shot-*.png`.
Without KVM (Docker Desktop on Windows has none) QEMU emulates the CPU: ~5 min to the kiosk, Chromium's first paint
minutes later – fine for a check, too slow for routine. In a VM PipeWire's own rule forces the quantum to 1024; the
deck config (48 kHz / 128) is read all the same (the allowed-rates line proves it) and applies on real hardware.
Host QEMU on Windows: `winget install SoftwareFreedomConservancy.QEMU`. VirtualBox works as well: a new Linux VM,
the ISO as an optical drive (then without persistence) or `VBoxManage convertfromraw out/vm/stick.img stick.vdi` as
a disk with EFI enabled.

## The boot menu

UEFI (systemd-boot) boots straight on – **hold / tap Space at power-on** for its menu; BIOS (syslinux) shows the menu
under the logo. The same three entries: **SCUTOONSDECK (live)** – the default;
**recheck the hardware** – `deck.recheck` on the kernel line, deck-check runs again on a stick that already knows the
machine; **safe graphics, messages stay on screen** – `nomodeset panic=0 loglevel=7 console=tty1`, no serial console:
for a machine that resets or shows nothing (the kernel's messages stay readable, a panic does not reboot, no boot
splash: `plymouth.enable=0`). Plus "Boot
the existing OS", Reboot, Power Off on BIOS. Nothing on the stick installs anything by itself.

## What happens at boot (tty1)

Under the logo (owner 2026-10-29, #179): the SCUTOONS logo in white on black from the initramfs on (plymouth, theme
`rootfs/usr/share/plymouth/themes/scutoons`, a quiet kernel line: `quiet splash loglevel=3 …`). deck-firstboot's
steps show as messages under it; it steps aside only for a menu (the Wi-Fi menu when offline or asked for, a failing
check's shell) and quits, keeping its last frame, right before cage starts (deck-kiosk.service); the stage's first
frame is the logo again (deck/stage: until the chapter and the music are loaded), and F2 keeps it up until the
browser's window is there.

1. `deck-firstboot.service` – persistence (`deck-persist`), the defaults into `/var/lib/deck` (`venue.toml`,
   `deck.toml`), the local relay starts.
2. `deck-wifi` – online already (wired, a remembered Wi-Fi) → 8 s countdown (under the splash: a message, any key
   opens the menu); else **W** nmtui's picker, **H** the deck's
   own hotspot (`deck-hotspot.nmconnection`: SSID `scutoons-deck`, key `scutoons-deck`, the deck is 10.42.0.1),
   **N** no network, **S** a shell.
3. `deck-check` – the report on screen, `/var/lib/deck/machine.toml` (class + features; CYBERDECKS.md §1). Once per
   stick; `deck.recheck` on the kernel line or `deck-check` in a shell re-runs it. Everything degrades with a warning
   (no Vulkan, software Vulkan, no audio device = the feature off, the kiosk still starts); only a machine without any
   display device leaves a root shell instead of the kiosk.
   Then the deck's key and its code to pair with ("Pairing" below); `deck-link.service` keeps it paired.
4. `deck-kiosk.service` – `cage` runs `deck-session` as the user `deck`, which plays `kiosk` from
   `/var/lib/deck/deck.toml`: **`stage`** (the default) = `deck-stage` (deck/stage/README.md) with `scene` (empty = the
   built-in "AV madness": catpope 1's slices, captions and music + synths, the FFT drives the picture; its cache in
   `/var/lib/deck/cache`); **F2** opens Chromium on the page `conductor` names (empty = the deck's own start page: the
   machine report, the relay's `/status`, the link to the conductor) on top of the stage, which shows the logo until
   Chromium's window covers it, then pauses and mutes; closing Chromium (Ctrl+Shift+W) shows the stage at once. A
   stage that fails opens Chromium alone.
   `kiosk = "browser"` = Chromium only. `deck-bridge.service` joins the local relay with `/var/lib/deck/venue.toml`.
   The stage's relay: deck.toml's `relay`, empty = the hosted `wss://live.scutoons.com/live/CATH` (#199) – joined only
   when the scene joins a session (`deck-set --restart scene cathedral`) or an outlet is set, so a deck needs no typed
   address; `deck-set relay ws://<host>:7700/live/<code>` joins a venue's relay instead.

tty2 is a root shell without a login (Ctrl+Alt+F2 – getty@tty2 autologin; cage runs with `-s`, VT switching on; Ctrl+Alt+F1 back); `deck-set key value` replaces a deck.toml setting (`deck-set --restart scene cathedral` plays it at once), the stage powers off on
Ctrl+Alt+Del and logind on a short press of the power button, so is the serial port; ssh is key-only (no passwords on the image). `pacman -S` works
on the live stick (the cache lives on DECKDATA); the live image overrides mkinitcpio's install hook to do nothing on the booted stick (it still runs in the build), since
the kernel and initramfs stay in the ISO (an installed deck gets the stock hook back). The user `deck`
(uid 1000) is in audio, video, input, realtime, uucp; PipeWire runs at 48 kHz / quantum 128 with rtkit
(`rootfs/etc/pipewire/pipewire.conf.d/10-deck-rt.conf`) under `user@1000` (linger) so the check can read it.

## Pairing (#180)

A deck touches SCUTOONS data only once a member approved it (CYBERDECKS.md §9). `deck-link` (deck/link) does it:

1. **Identity:** `deck-firstboot` runs `deck-link id` – the deck's Ed25519 key in `/var/lib/deck/identity` (0600 root,
   on DECKDATA; an installed deck: the root fs). A stick without persistence makes a new key each boot and asks again.
2. **The code:** `deck-link once` asks scutoons.com and puts `pair this deck: code K7Q-42M · https://scutoons.com/account/decks?deck=K7Q42M`
   under the boot logo; the stage shows the code + a QR in its corner while the deck waits (scan it with the phone,
   signed in, approve), the start page (F2) and `deck-link status` on tty2 say it too.
3. **The daemon:** `deck-link.service` (`deck-link run`, root) keeps asking, then holds the deck's ~1 h token in
   `/run/deck-link/token` (2750 root:deck from tmpfiles.d, the token 0640 – root and the user `deck` read it), renews
   it, and after a revoke / expiry goes back to a new code by itself. The stage sends it to the API (scene-content, the
   library, show media); built-in scenes play before approval.

## wss in the venue (#201)

A phone on https://scutoons.com/live/deck may not open `ws://192.168.…`, so the deck's relay also speaks **wss on the
same port 7700** once it has the venue certificate (`*.lan.scutoons.com`, Let's Encrypt). Its name needs no
registration: bunny DNS answers `<LAN IP with dashes>.lan.scutoons.com` with that address (deploy/bunny/lan-dns.js,
private addresses only), so the X1 at 192.168.1.116 is `wss://192-168-1-116.lan.scutoons.com:7700`.

- `deck-tls` (deck-tls.timer: 90 s after boot, then every 6 h; or by hand on tty2 as root) fetches the pair with the
  deck's own token (`GET /api/decks/lan-cert?format=pem` – no admin token on a deck) into
  `/var/lib/deck/tls/{fullchain.pem,privkey.pem}` (root:deck, the key 0640). Not paired yet → it says so and the timer
  tries again. The first pair restarts `deck-relay`; a renewed one is used from the next connection on (no restart).
- `deck-tls --status`: the certificate's names + expiry and the deck's wss addresses; `deck-tls --names` only those.
- `journalctl -u deck-relay` says `relay listening (ws + wss)`; from a laptop on the LAN:
  `curl https://192-168-1-116.lan.scutoons.com:7700/status`.

`deck-relay.service` starts with the certificate when both files are readable, else plain ws; a wss relay that fails
(a broken pair) is followed by a plain one. The owner's whole path (bunny zone, the workflow, the router's DNS-rebind
exception): research/LIVE.md B.4.

## Install to a partition

`sudo deck-install` on the stick (a shell on tty2 or the serial console). It lists the machine's partitions with their
type, file system and label (the stick excluded), the user picks one by number and confirms by typing its name (an ESP
or a Windows / BitLocker partition is refused), optionally picks an EFI system partition – each shown with its free
space and whether it holds Windows' boot manager; an empty one is formatted as FAT32 `DECKESP` on request, Windows' own
needs a `yes`, one with less than 150 MB free is refused. Every question comes before the first write. Then mkfs.ext4 →
unsquashfs the live root → the live-only bits removed → the kernel from `/usr/lib/modules/*/vmlinuz` to
`/boot/vmlinuz-<pkgbase>` (mkarchiso empties the root's /boot – without it mkinitcpio has no kernel) → fstab →
mkinitcpio → a systemd-boot entry on the ESP, and the boot-order advice. It never creates or resizes partitions: make
them first (cfdisk). The installed deck boots the same flow (the check runs once, then the kiosk).

Next to Windows (the X1, #200): an ESP of its own (256M, type EFI System – 100 MB is the real floor, the installer refuses
< 150 MB free) next to Windows' – Windows' ESP stays untouched; Windows then starts with F12 → Windows Boot Manager. The
firmware entry is written by `bootctl` run on the live system, not in the chroot (only there it reaches the EFI
variables). **Run on the owner's X1 on 2026-11-01** (s7 stick, 16.4 GB root + the 256M ESP in the space freed from C:):
it installed, the deck booted from the NVMe, the Wi-Fi was remembered. Findings of that run, all fixed here: with
`copytoram` (archiso's default at 16 GB RAM) the root image is in `/run/archiso/copytoram/`, not on the stick; the first
run had no boot entry in the firmware (bootctl in the chroot) – from the shell `mount /dev/nvme0n1p4 /mnt/esp && bootctl
--esp-path=/mnt/esp install` registers it. A restart of `deck-kiosk` from the boot menu's own shell hangs (the kiosk
requires deck-firstboot, which still waits on that shell) – `exit` it instead.

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

the pacman repository (`pkg.scutoons.com`, the PKGBUILD built by
CI) · the ARM images (phase 3, #174) · `deck-install` on real hardware · the persistence partition on a real stick
(the MBR append was verified by hand in the VM, not yet as a full first boot + reboot) · the ThinkPad X1 row in
HARDWARE.md (owner).
