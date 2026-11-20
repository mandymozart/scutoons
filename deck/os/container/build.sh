#!/usr/bin/env bash
# Runs INSIDE the archlinux:latest container (deck/os/build.sh starts it; --privileged for pacstrap's chroot).
#   /src   = deck/ (read-only)        /work = a named volume (cargo target, the profile, mkarchiso's work dir)
#   /out   = deck/os/out on the host  /var/cache/pacman/pkg = a named volume (the package cache between builds)
# Steps: packages → the deck binaries (cargo, release; the stage with PipeWire) → releng + deck/os/profile + deck/os/rootfs → mkarchiso → /out.
# Env: DECK_PROFILE (full: the deck + the editor, every module; lean: the deck only, its features from its scenes – os/profiles.toml),
# DECK_KERNEL (linux-rt; `linux` when the rt kernel is unavailable), DECK_SKIP_CARGO=1 (reuse /work/target),
# DECK_BINARIES_ONLY=1 (stop after cargo: the binaries into /out/bin – `npm run deck:update`, deck/os/update.sh).
set -euo pipefail
[ "${DECK_TRACE:-0}" = 1 ] && set -x

KERNEL=${DECK_KERNEL:-linux-rt}
SRC=/src; WORK=/work; OUT=/out
PROFILE=$WORK/profile
RELENG=/usr/share/archiso/configs/releng

echo "== packages (host tools)"
pacman-key --init >/dev/null 2>&1 || true
# pipewire + clang: the stage's PipeWire backend (pipewire-sys runs bindgen against libpipewire's headers);
# libva + mesa (libgbm) + libdrm: hardware video decoding (deck-media-hw; cros-libva runs bindgen against libva's headers);
# librsvg + imagemagick: the logo's PNGs (plymouth, the BIOS menu) from the SVG
pacman -Syu --noconfirm --needed archiso rust base-devel alsa-lib git pipewire clang libva mesa libdrm librsvg imagemagick >/dev/null
echo "   archiso $(pacman -Q archiso | cut -d' ' -f2), rust $(rustc --version | cut -d' ' -f2), kernel package: $KERNEL"
pacman -Si "$KERNEL" >/dev/null 2>&1 || { echo "no package '$KERNEL' in the repos – set DECK_KERNEL=linux"; exit 1; }
# cros-libva 0.0.12 (deck-media-hw) does not compile against libva >= 2.23's headers (new VP9 encode fields): its
# bindings come from libva 2.22's headers, pinned by hash (the ABI is the same; the binaries link the system's libva)
VA_H=$WORK/libva-2.22.0-headers
if [ ! -f "$VA_H/usr/include/va/va_version.h" ]; then
  curl -fsSL -o /tmp/libva-h.pkg.tar.zst https://archive.archlinux.org/packages/l/libva/libva-2.22.0-1-x86_64.pkg.tar.zst
  echo "ad3083a252bbac119d405ebeea7d86f9fc8e79cc27759303300e55ac9f6ffbda  /tmp/libva-h.pkg.tar.zst" | sha256sum -c --quiet
  mkdir -p "$VA_H" && bsdtar -xf /tmp/libva-h.pkg.tar.zst -C "$VA_H" usr/include
fi
export CROS_LIBVA_H_PATH=$VA_H/usr/include

CLASS=${DECK_PROFILE:-full}
cargo_in_src() { (cd "$SRC" && CARGO_TARGET_DIR=$WORK/target CARGO_HOME=$WORK/cargo-home cargo "$@"); }
# the deck's features: the class's platform and modules + what the scenes it carries need (the sandboxes only on a dev image)
scenes=(native/scenes); [ "${DECK_SANDBOXES:-}" = 1 ] && scenes+=(native/scenes/sandbox)
FEATURES=$(cargo_in_src run -q --release --locked -p deck-scene-features -- --profile os/profiles.toml "$CLASS" "${scenes[@]}")
APPS=$(cargo_in_src run -q --release --locked -p deck-scene-features -- --apps os/profiles.toml "$CLASS" | tr ',' ' ')
BINS="relay bridge deck-check deck deck-link deck-update $APPS"
echo "== the deck binaries, $CLASS: relay, bridge, deck-check, deck ($FEATURES), deck-link, deck-update${APPS:+, $APPS} – release, Linux x86_64"
if [ "${DECK_SKIP_CARGO:-0}" != 1 ]; then
  # three cargo lines: --no-default-features applies to every -p, and an app beside the deck must not add features to it
  {
    cargo_in_src build --release --locked -p relay -p bridge -p deck-check -p deck-link -p deck-update &&
      cargo_in_src build --release --locked -p deck-native --no-default-features --features "$FEATURES" &&
      if [ -n "$APPS" ]; then cargo_in_src build --release --locked $(printf -- '-p %s ' $APPS) --features "$(echo "$FEATURES" | sed 's/[^,]*/deck-native\/&/g')"; fi
  } >"$OUT/cargo.log" 2>&1 || { echo "cargo failed – the last lines (all: deck/os/out/cargo.log):"; tail -40 "$OUT/cargo.log"; exit 1; }
  tail -3 "$OUT/cargo.log"
fi
for b in $BINS; do [ -x "$WORK/target/release/$b" ] || { echo "missing $WORK/target/release/$b"; exit 1; }; done
editor=0; case " $APPS " in *" deck-editor "*) editor=1;; esac
if [ "${DECK_BINARIES_ONLY:-0}" = 1 ]; then
  # a remote update's binaries (#237): deck/os/update.sh bundles them with deck/os/rootfs on the host
  rm -rf "$OUT/bin"; mkdir -p "$OUT/bin"
  for b in $BINS; do install -m755 "$WORK/target/release/$b" "$OUT/bin/$b"; done
  if [ $editor = 1 ]; then
    # the editor reads its design tokens beside its binary
    install -m644 "$SRC/editor/tokens.toml" "$OUT/bin/tokens.toml"
    # the editor's demo shows, their rigs, book and story events: the bundle writes them to /usr/share/deck/demos
    mkdir -p "$OUT/bin/demos" && cp -r "$SRC/demos/." "$OUT/bin/demos/"
  fi
  echo "== done: $OUT/bin"; exit 0
fi

echo "== the profile: releng + deck/os/profile + deck/os/rootfs"
rm -rf "$PROFILE"; cp -r "$RELENG" "$PROFILE"
# releng's identity → ours (the rest of profiledef.sh – buildmodes, bootmodes, image options – stays releng's)
sed -i -e '/^iso_name=/d' -e '/^iso_label=/d' -e '/^iso_publisher=/d' -e '/^iso_application=/d' -e '/^iso_version=/d' "$PROFILE/profiledef.sh"
cat "$SRC/os/profile/profiledef.deck.sh" >>"$PROFILE/profiledef.sh"
# releng's live-only bits we replace: root autologin + its script, the mirror chooser, reflector, networkd; the wants
# list is rebuilt below from what the deck ships
rm -rf "$PROFILE/airootfs/etc/systemd/system/getty@tty1.service.d" "$PROFILE/airootfs/root/.automated_script.sh" \
       "$PROFILE/airootfs/root/.zlogin" "$PROFILE/airootfs/etc/systemd/system/multi-user.target.wants" \
       "$PROFILE/airootfs/etc/systemd/system/choose-mirror.service" "$PROFILE/airootfs/etc/systemd/scripts" \
       "$PROFILE/airootfs/etc/xdg/reflector" "$PROFILE/airootfs/etc/systemd/network" \
       "$PROFILE/airootfs/etc/systemd/system/systemd-networkd-wait-online.service.d"
# the boot menus are ours (SCUTOONSDECK, no install-medium / speech / memtest entries, no PXE, no grub – the bootmodes are
# bios.syslinux + uefi.systemd-boot); the BIOS menu's splash.png is the logo (below)
rm -rf "$PROFILE/efiboot/loader/entries" "$PROFILE/grub" "$PROFILE/syslinux/archiso_pxe.cfg" "$PROFILE/syslinux/archiso_pxe-linux.cfg"
cp -r "$SRC/os/profile/packages.x86_64" "$SRC/os/profile/pacman.conf" "$PROFILE/"
cp -r "$SRC/os/profile/efiboot" "$SRC/os/profile/syslinux" "$PROFILE/"
cp -r "$SRC/os/profile/airootfs/." "$PROFILE/airootfs/"
cp -r "$SRC/os/rootfs/." "$PROFILE/airootfs/"
# a NetworkManager keyfile with an empty value (`interface-name=`) is rejected whole – the profile never loads
if grep -nE '^[A-Za-z0-9_.-]+=$' "$PROFILE"/airootfs/etc/NetworkManager/system-connections/*.nmconnection; then
  echo "an empty value in a NetworkManager keyfile (above) – NetworkManager would drop the profile"; exit 1
fi
# the logo (#179) – the official SVG in white (deck/native/assets) → plymouth's logo.png and the BIOS menu's background
logo="$SRC/native/assets/SCUTOONS-logo-white.svg"
rsvg-convert -w 1536 "$logo" -o "$PROFILE/airootfs/usr/share/plymouth/themes/scutoons/logo.png"
rsvg-convert -w 320 "$logo" -o "$WORK/logo-320.png"
magick -size 640x480 xc:black "$WORK/logo-320.png" -gravity north -geometry +0+24 -composite PNG24:"$PROFILE/syslinux/splash.png"
install -Dm755 "$WORK/target/release/relay" "$PROFILE/airootfs/usr/bin/deck-relay"
install -Dm755 "$WORK/target/release/bridge" "$PROFILE/airootfs/usr/bin/deck-bridge"
install -Dm755 "$WORK/target/release/deck-check" "$PROFILE/airootfs/usr/bin/deck-check"
install -Dm755 "$WORK/target/release/deck" "$PROFILE/airootfs/usr/bin/deck"
install -Dm755 "$WORK/target/release/deck-link" "$PROFILE/airootfs/usr/bin/deck-link"
install -Dm755 "$WORK/target/release/deck-update" "$PROFILE/airootfs/usr/bin/deck-update"
if [ $editor = 1 ]; then
  # where an update bundle puts them too (deck/update bundle.rs); deck-session starts the editor only where it is installed
  install -Dm755 "$WORK/target/release/deck-editor" "$PROFILE/airootfs/usr/bin/deck-editor"
  install -Dm644 "$SRC/editor/tokens.toml" "$PROFILE/airootfs/usr/share/deck/editor/tokens.toml"
  mkdir -p "$PROFILE/airootfs/usr/share/deck/demos" && cp -r "$SRC/demos/." "$PROFILE/airootfs/usr/share/deck/demos/"
fi
# #427: the system scenes are in the binary; the sandboxes (checks, demos) only on a dev image – DECK_SANDBOXES=1 – where
# F5 lists them (the deck's dev mode); a release image carries none
if [ "${DECK_SANDBOXES:-}" = 1 ]; then
  install -Dm644 -t "$PROFILE/airootfs/usr/share/deck/scenes/sandbox" "$SRC"/native/scenes/sandbox/*.toml
fi
chmod 755 "$PROFILE/airootfs/usr/bin/"deck-*
wants="$PROFILE/airootfs/etc/systemd/system/multi-user.target.wants"; mkdir -p "$wants"
for u in NetworkManager iwd avahi-daemon sshd deck-firstboot deck-kiosk deck-relay deck-bridge deck-link; do
  ln -sf "/usr/lib/systemd/system/$u.service" "$wants/$u.service"
done
# the venue certificate now and then (#201)
mkdir -p "$PROFILE/airootfs/etc/systemd/system/timers.target.wants"
ln -sf /usr/lib/systemd/system/deck-tls.timer "$PROFILE/airootfs/etc/systemd/system/timers.target.wants/deck-tls.timer"
# the deck's network is NetworkManager: Arch's presets enable systemd-networkd + its wait-online, which waits for links
# networkd never manages and held network-online.target for 2 min each boot (the X1, 2026-11-01, #251) – masked
ln -sf /dev/null "$PROFILE/airootfs/etc/systemd/system/systemd-networkd.service"
ln -sf /dev/null "$PROFILE/airootfs/etc/systemd/system/systemd-networkd-wait-online.service"
# remote updates (#237)
ln -sf /usr/lib/systemd/system/deck-update.timer "$PROFILE/airootfs/etc/systemd/system/timers.target.wants/deck-update.timer"
# releng's pacman-init (the live keyring) lives in /etc – keep it while releng ships it
[ -e "$PROFILE/airootfs/etc/systemd/system/pacman-init.service" ] && ln -sf /etc/systemd/system/pacman-init.service "$wants/pacman-init.service"
# releng's file_permissions name a file we removed
sed -i '/automated_script/d' "$PROFILE/profiledef.sh"
[ $editor = 1 ] || sed -i '\|"/usr/bin/deck-editor"|d' "$PROFILE/profiledef.sh"
# the serial console on ttyS0 (the VM test, a cable on a real deck) is NOT enabled here: systemd's serial-getty generator
# starts it only where console=ttyS0 is on the kernel line (the live entries, profiledef). An installed deck without a
# serial port then never runs it – a hard enable restarted it forever on the X1 (672 times, #262)
date -u +%Y.%m.%d >"$PROFILE/airootfs/etc/deck-release"
# what the root holds – a remote update is newer when its build is higher (#237, deck-update)
printf '{"version":"image %s","build":%s}\n' "$(date -u +%Y.%m.%d)" "$(date -u +%s)" >"$PROFILE/airootfs/usr/share/deck/build.json"
: >"$PROFILE/airootfs/etc/deck-live"
# the kernel: releng names linux everywhere (boot entries, the mkinitcpio preset) – rename to $KERNEL
if [ "$KERNEL" != linux ]; then
  sed -i "s/^linux-rt$/$KERNEL/" "$PROFILE/packages.x86_64"
  { grep -rl -e 'vmlinuz-linux' -e 'initramfs-linux' "$PROFILE/grub" "$PROFILE/syslinux" "$PROFILE/efiboot" 2>/dev/null || true; } \
    | xargs -r sed -i -e "s|vmlinuz-linux\b|vmlinuz-$KERNEL|g" -e "s|initramfs-linux\.img|initramfs-$KERNEL.img|g" -e "s|initramfs-linux-fallback|initramfs-$KERNEL-fallback|g"
  if [ -f "$PROFILE/airootfs/etc/mkinitcpio.d/linux.preset" ]; then
    mv "$PROFILE/airootfs/etc/mkinitcpio.d/linux.preset" "$PROFILE/airootfs/etc/mkinitcpio.d/$KERNEL.preset"
  fi
else
  sed -i 's/^linux-rt$/linux/' "$PROFILE/packages.x86_64"
fi
echo "   kernel: $(grep -E '^linux' "$PROFILE/packages.x86_64" | tr '\n' ' ')"

echo "== mkarchiso"
rm -rf "$WORK/iso"; mkdir -p "$OUT"
mkarchiso -v -w "$WORK/iso" -o "$OUT" "$PROFILE" >"$WORK/mkarchiso.log" 2>&1 || { tail -40 "$WORK/mkarchiso.log"; exit 1; }
iso=$(ls -t "$OUT"/scutoonsdeck-*-x86_64.iso | head -1)
if [ "$CLASS" != full ]; then mv "$iso" "${iso/scutoonsdeck-/scutoonsdeck-$CLASS-}"; iso=${iso/scutoonsdeck-/scutoonsdeck-$CLASS-}; fi
cp "$WORK/mkarchiso.log" "$OUT/mkarchiso.log"
ls -l "$iso"
echo "== done: $iso"
