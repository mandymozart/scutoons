#!/usr/bin/env bash
# deck/os/vm/run.sh – boot the newest deck ISO like a USB stick in QEMU: `npm run deck:vm [-- bios|uefi] [-- --docker]`.
# The ISO is copied to a stick image with 4 GiB of free space after it (deck-persist creates DECKDATA there on the
# first boot, so a second boot shows persistence). Audio (intel-hda), USB (xhci + keyboard + tablet), a wired NIC
# (NetworkManager goes online by itself), a serial console on ttyS0 (a root shell), 4 GB RAM, 2 cpus.
#   host mode   : qemu-system-x86_64 on PATH (Windows: `winget install SoftwareFreedomConservancy.QEMU`,
#                 C:\Program Files\qemu is tried too) – a window opens; UEFI needs OVMF next to qemu (share/edk2-x86_64-code.fd).
#   --docker    : QEMU inside an archlinux container, no window: the serial console on the terminal, screenshots every
#                 15 s to deck/os/out/vm/shot-*.png (the kiosk is visible there), monitor on deck/os/out/vm/monitor.
#                 KVM when Docker exposes /dev/kvm (WSL2 nested virtualization), else TCG (slow: ~5 min to the kiosk).
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd); os=$(cd "$here/.." && pwd)
mode=uefi; docker=0
for a in "$@"; do case "$a" in bios) mode=bios ;; uefi) mode=uefi ;; --docker) docker=1 ;; esac; done
iso=$(ls -t "$os"/out/deck-*-x86_64.iso 2>/dev/null | head -1) || true
[ -n "${iso:-}" ] || { echo "no ISO in deck/os/out – npm run deck:iso first"; exit 1; }
mkdir -p "$os/out/vm"
stick="$os/out/vm/stick.img"
if [ ! -f "$stick" ] || [ "$iso" -nt "$stick" ]; then
  echo "stick image from $(basename "$iso") + 4 GiB free"
  cp "$iso" "$stick"
  if command -v truncate >/dev/null 2>&1; then truncate -s +4G "$stick"; else python3 - "$stick" <<'EOF'
import os, sys; p = sys.argv[1]; os.truncate(p, os.path.getsize(p) + 4 * 1024 ** 3)
EOF
  fi
fi

common=(-m 4G -smp 2 -rtc base=utc
  -drive "if=none,id=stick,format=raw,file=STICK" -device qemu-xhci,id=xhci -device usb-storage,bus=xhci.0,drive=stick,bootindex=0
  -device usb-kbd -device usb-tablet
  -device intel-hda -device hda-duplex
  -nic user,model=virtio-net-pci
  -device virtio-vga
  -serial SERIAL)

if [ "$docker" = 1 ]; then
  out_m=$(command -v cygpath >/dev/null 2>&1 && cygpath -m "$os/out" || echo "$os/out")
  kvm=(); [ -e /dev/kvm ] && kvm=(--device /dev/kvm)
  args=("${common[@]}"); args=("${args[@]/STICK//out/vm/stick.img}"); args=("${args[@]/SERIAL/mon:stdio}")
  echo "QEMU in Docker ($mode) – serial console here, screenshots in deck/os/out/vm/ (Ctrl+C ends it)"
  MSYS_NO_PATHCONV=1 docker run --rm -it "${kvm[@]}" -v "$out_m:/out" -v scutoons-deck-pkg:/var/cache/pacman/pkg \
    -e MODE="$mode" -e ARGS="${args[*]}" "${DECK_IMAGE:-archlinux:latest}" bash -c '
      pacman -Syu --noconfirm --needed qemu-base edk2-ovmf >/dev/null
      accel=tcg; [ -e /dev/kvm ] && accel=kvm
      fw=(); [ "$MODE" = uefi ] && fw=(-drive if=pflash,format=raw,readonly=on,file=/usr/share/edk2/x64/OVMF_CODE.4m.fd)
      rm -f /out/vm/shot-*.png /out/vm/monitor
      (sleep 20; n=0; while [ -S /out/vm/monitor ]; do n=$((n+1)); printf "screendump /out/vm/shot-%02d.png -f png\n" $n | socat - UNIX-CONNECT:/out/vm/monitor >/dev/null 2>&1 || true; sleep 15; done) &
      exec qemu-system-x86_64 -accel $accel -cpu max "${fw[@]}" -display none -monitor unix:/out/vm/monitor,server,nowait $ARGS'
  exit
fi

qemu=$(command -v qemu-system-x86_64 || true)
[ -n "$qemu" ] || for p in "/c/Program Files/qemu/qemu-system-x86_64.exe" "/usr/bin/qemu-system-x86_64"; do [ -x "$p" ] && qemu=$p && break; done
[ -n "$qemu" ] || { echo "no qemu-system-x86_64 – install QEMU or use --docker"; exit 1; }
stick_m=$(command -v cygpath >/dev/null 2>&1 && cygpath -m "$stick" || echo "$stick")
args=("${common[@]}"); args=("${args[@]/STICK/$stick_m}"); args=("${args[@]/SERIAL/stdio}")
fw=()
if [ "$mode" = uefi ]; then
  for f in "$(dirname "$qemu")/share/edk2-x86_64-code.fd" /usr/share/edk2/x64/OVMF_CODE.4m.fd /usr/share/OVMF/OVMF_CODE.fd /usr/share/edk2-ovmf/x64/OVMF_CODE.fd; do
    [ -f "$f" ] && fw=(-drive "if=pflash,format=raw,readonly=on,file=$(command -v cygpath >/dev/null 2>&1 && cygpath -m "$f" || echo "$f")") && break
  done
  [ ${#fw[@]} -gt 0 ] || echo "no OVMF firmware found – booting BIOS instead"
fi
accel=(); case "$(uname -s)" in Linux) [ -w /dev/kvm ] && accel=(-accel kvm) ;; MINGW*|MSYS*|CYGWIN*) accel=(-accel whpx,kernel-irqchip=off -accel tcg) ;; Darwin) accel=(-accel hvf) ;; esac
echo "QEMU ($mode): $(basename "$iso") as a USB stick – the serial console is this terminal"
exec "$qemu" "${accel[@]}" -cpu max "${fw[@]}" "${args[@]}"
