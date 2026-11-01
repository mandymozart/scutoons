# Appended to releng's profiledef.sh by container/build.sh (the identity lines of releng are removed first; buildmodes,
# bootmodes and the image options stay releng's, so the profile follows archiso upstream). CYBERDECKS.md §3, #173.
iso_name="scutoonsdeck"
iso_label="SCUTOONSDECK"
iso_publisher="SCUTOONS <https://scutoons.com>"
iso_application="SCUTOONS deck live"
# a serial console next to tty1 (the VM test, a cable on a real deck); harmless without a port. A quiet boot under the
# logo (plymouth, #179): `splash`, no kernel / udev chatter, no cursor; plymouth ignores the serial console so the logo
# shows on the screen (the serial line still gets the messages). The "safe" entries turn plymouth off again.
kernel_params_x86_64="console=ttyS0,115200 console=tty1 quiet splash loglevel=3 rd.udev.log_level=3 vt.global_cursor_default=0 plymouth.ignore-serial-consoles"
iso_version="$(date --date="@${SOURCE_DATE_EPOCH:-$(date +%s)}" +%Y.%m.%d)"
file_permissions+=(
  ["/usr/bin/deck-firstboot"]="0:0:755"
  ["/usr/bin/deck-wifi"]="0:0:755"
  ["/usr/bin/deck-persist"]="0:0:755"
  ["/usr/bin/deck-set"]="0:0:755"
  ["/usr/bin/deck-tls"]="0:0:755"
  ["/usr/bin/deck-session"]="0:0:755"
  ["/usr/bin/deck-install"]="0:0:755"
  ["/usr/bin/deck-check"]="0:0:755"
  ["/usr/bin/deck-relay"]="0:0:755"
  ["/usr/bin/deck-bridge"]="0:0:755"
  ["/usr/bin/deck-stage"]="0:0:755"
  ["/usr/bin/deck-link"]="0:0:755"
  ["/usr/bin/deck-update"]="0:0:755"
  ["/etc/NetworkManager/system-connections/deck-hotspot.nmconnection"]="0:0:600"
)
