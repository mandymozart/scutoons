# Appended to releng's profiledef.sh by container/build.sh (the identity lines of releng are removed first; buildmodes,
# bootmodes and the image options stay releng's, so the profile follows archiso upstream). CYBERDECKS.md §3, #173.
iso_name="deck"
iso_label="DECK_$(date --date="@${SOURCE_DATE_EPOCH:-$(date +%s)}" +%Y%m)"
iso_publisher="SCUTOONS <https://scutoons.com>"
iso_application="SCUTOONS deck live"
iso_version="$(date --date="@${SOURCE_DATE_EPOCH:-$(date +%s)}" +%Y.%m.%d)"
file_permissions+=(
  ["/usr/bin/deck-firstboot"]="0:0:755"
  ["/usr/bin/deck-wifi"]="0:0:755"
  ["/usr/bin/deck-persist"]="0:0:755"
  ["/usr/bin/deck-session"]="0:0:755"
  ["/usr/bin/deck-install"]="0:0:755"
  ["/usr/bin/deck-check"]="0:0:755"
  ["/usr/bin/deck-relay"]="0:0:755"
  ["/usr/bin/deck-bridge"]="0:0:755"
  ["/etc/NetworkManager/system-connections/deck-hotspot.nmconnection"]="0:0:600"
)
