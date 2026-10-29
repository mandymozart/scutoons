#!/usr/bin/env bash
# Runs INSIDE the archlinux:latest container (deck/os/build.sh starts it; --privileged for pacstrap's chroot).
#   /src   = deck/ (read-only)        /work = a named volume (cargo target, the profile, mkarchiso's work dir)
#   /out   = deck/os/out on the host  /var/cache/pacman/pkg = a named volume (the package cache between builds)
# Steps: packages → the deck binaries (cargo, release) → releng + deck/os/profile + deck/os/rootfs → mkarchiso → /out.
# Env: DECK_KERNEL (linux-rt; `linux` when the rt kernel is unavailable), DECK_SKIP_CARGO=1 (reuse /work/target).
set -euo pipefail
[ "${DECK_TRACE:-0}" = 1 ] && set -x

KERNEL=${DECK_KERNEL:-linux-rt}
SRC=/src; WORK=/work; OUT=/out
PROFILE=$WORK/profile
RELENG=/usr/share/archiso/configs/releng

echo "== packages (host tools)"
pacman-key --init >/dev/null 2>&1 || true
pacman -Syu --noconfirm --needed archiso rust base-devel alsa-lib git >/dev/null
echo "   archiso $(pacman -Q archiso | cut -d' ' -f2), rust $(rustc --version | cut -d' ' -f2), kernel package: $KERNEL"
pacman -Si "$KERNEL" >/dev/null 2>&1 || { echo "no package '$KERNEL' in the repos – set DECK_KERNEL=linux"; exit 1; }

echo "== the deck binaries (relay, bridge, deck-check) – release, Linux x86_64"
if [ "${DECK_SKIP_CARGO:-0}" != 1 ]; then
  (cd "$SRC" && CARGO_TARGET_DIR=$WORK/target CARGO_HOME=$WORK/cargo-home cargo build --release --locked -p relay -p bridge -p deck-check 2>&1 | tail -3)
fi
for b in relay bridge deck-check; do [ -x "$WORK/target/release/$b" ] || { echo "missing $WORK/target/release/$b"; exit 1; }; done

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
cp -r "$SRC/os/profile/packages.x86_64" "$SRC/os/profile/pacman.conf" "$PROFILE/"
cp -r "$SRC/os/profile/airootfs/." "$PROFILE/airootfs/"
cp -r "$SRC/os/rootfs/." "$PROFILE/airootfs/"
install -Dm755 "$WORK/target/release/relay" "$PROFILE/airootfs/usr/bin/deck-relay"
install -Dm755 "$WORK/target/release/bridge" "$PROFILE/airootfs/usr/bin/deck-bridge"
install -Dm755 "$WORK/target/release/deck-check" "$PROFILE/airootfs/usr/bin/deck-check"
chmod 755 "$PROFILE/airootfs/usr/bin/"deck-*
wants="$PROFILE/airootfs/etc/systemd/system/multi-user.target.wants"; mkdir -p "$wants"
for u in NetworkManager iwd avahi-daemon sshd deck-firstboot deck-kiosk deck-relay; do
  ln -sf "/usr/lib/systemd/system/$u.service" "$wants/$u.service"
done
# releng's pacman-init (the live keyring) lives in /etc – keep it while releng ships it
[ -e "$PROFILE/airootfs/etc/systemd/system/pacman-init.service" ] && ln -sf /etc/systemd/system/pacman-init.service "$wants/pacman-init.service"
# releng's file_permissions name a file we removed
sed -i '/automated_script/d' "$PROFILE/profiledef.sh"
# a serial console on ttyS0 (the VM test, a cable on a real deck); no password = key-only ssh, root on serial only
mkdir -p "$PROFILE/airootfs/etc/systemd/system/getty.target.wants"
ln -sf /usr/lib/systemd/system/serial-getty@.service "$PROFILE/airootfs/etc/systemd/system/getty.target.wants/serial-getty@ttyS0.service"
date -u +%Y.%m.%d >"$PROFILE/airootfs/etc/deck-release"
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
iso=$(ls -t "$OUT"/deck-*-x86_64.iso | head -1)
cp "$WORK/mkarchiso.log" "$OUT/mkarchiso.log"
ls -l "$iso"
echo "== done: $iso"
