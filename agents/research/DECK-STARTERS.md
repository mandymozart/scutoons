Status: research 2026-10-31 (owner: "possible hardware starter decks we can buy for cheap") – prices from web sources of 2026, check before buying; feeds deck/HARDWARE.md "Roles" and #174.

# Starter decks to buy – cheap hardware for each role

**The constraint today:** the deck image is **x86_64 only** (class A); the ARM images (class B / C) are phase 3 (#174).
So everything that should boot the stick *now* is an Intel / AMD machine. What we know works: the **ThinkPad X1 Carbon
Gen 7** (i7-8565U, Intel UHD 620) runs the cathedral at 60 fps, 1080p (deck/HARDWARE.md). Machines with the same GPU
generation or newer are safe bets; the stage needs Vulkan (Intel ANV covers Gen 9+), audio through PipeWire, and for
video a VA-API decoder.

## The candidates

| Starter | Price (2026) | Tier | Role it fits | Why | Watch out |
|---|---|---|---|---|---|
| **Used ThinkPad T480** (i5-8250U / 8350U, UHD 620, 8–16 GB) | median **€175** sold on eBay, most €145–242 | M | AV deck, conductor, live video (1080p) | the **same GPU as the X1** (proven), screen + keyboard + battery = a whole deck, swappable battery, 2 RAM slots, NVMe, HDMI, Ethernet; a lot of them around | asking prices run higher than sold prices; check the battery |
| **Intel N100 / N150 mini PC** (16 GB, 512 GB NVMe) | **~$140–160** new (N150 boxes like GMKtec); EU listings up to ~€255 | M | AV deck, musician, outlet on a projector | new, silent, ~7 W idle, the Xe GPU is about UHD 620 class, hardware decode H.264 / HEVC / VP9 / **AV1**, dual 4K out | single-channel RAM, 16 GB is the max; no screen / battery – a box for a projector or a stage; N150 ≈ N100 (<1 % apart) |
| **Used thin client: HP t640** (Ryzen R1505G) / **Dell Wyse 5070** (Pentium J5005) / Fujitsu Futro S740 | **€50–80** (t640), **~€49** (Wyse 5070) | S | light / device bridge, musician (sound only), a small outlet for slides | very cheap, passive or near silent, 5–8 W idle, x86 (boots our stick), the t640 has an NVMe slot | weak GPUs (3D only at low resolution), 4–8 GB RAM, the Wyse / Futro take SATA M.2 only; check the power supply is included |
| **Used X1 Carbon Gen 6** (i5/i7-8xxx) | rare under €250 (a €239 deal in 2024; refurb from ~£380) | M | AV deck, conductor | light, the X1 family we know | pricier than a T480 for the same insides |
| Raspberry Pi 5 | **8 GB ~$175, 16 GB ~$305** (list, Oct 2026 – memory prices drove three rises) | (C) | later: bridge / musician | Vulkan 1.3 (V3DV, conformant), tiny | **not for now:** ARM (needs #174), no longer cheap, **HEVC decode only – no H.264 hardware decode**; a used thin client is a third of the price and boots the stick today |
| RK3588 boards (Orange Pi 5 Plus, Radxa Rock 5B, Banana Pi M7) | ~$130–230 (16 GB Orange Pi 5 Plus ~$150), often imported | (B) | later: AV deck / outlet | 8 cores, Mali G610 – open Vulkan (PanVK, Vulkan 1.1 conformant, 1.2 advertised), a strong video block | **phase 3:** ARM image, mainline video decode on RK3588 not confirmed, stock is spotty – the class-B target for our own hardware, not a starter |

Phones (the choir, the audience) need no deck: the browser deck `/live/deck` (WebGL2 on the S22).

## With a screen built in – from watch to frame (owner 2026-10-31)
| Size | Device | Price | What it runs | Role |
|---|---|---|---|---|
| Watch | **LILYGO T-Watch S3 / S3 Plus** (ESP32-S3, 1.54″ touch, Wi-Fi; the Plus with a haptic motor, GPS, 940 mAh) | ~$40–60 | **no Linux, no browser** – a small Rust firmware (esp-rs) speaking the relay's JSON over Wi-Fi: cues, the voice part, a buzz on the beat, the being's mood as a face | a choir singer's / performer's wrist cue – a tier below S (protocol only, no stage); Wear OS watches have no usable browser |
| Phone | any phone with Chrome / WebGL2 (the S22 works), cheap Android, old phones | €0–150 | the browser deck `/live/deck` | choir, performers, audience |
| Tablet / frame | an Android tablet in kiosk mode | ~€100–200 | the browser deck or a show view | a lectern or small exhibit screen; photo frames are mostly closed Android – a tablet is the reliable "frame" |
| Handheld | **Steam Deck LCD, certified refurbished by Valve** (x86 Zen 2 + RDNA 2, 7″, controls, battery) | **€299** (64 GB) / €339 / €379 | boots our stick: the native deck with screen, buttons, sticks, audio | a performer's own deck – the beings' knobs on real sticks |
| Laptop | used ThinkPad T480 (above) | ~€175 | the native deck | conductor, AV deck |

## The smallest box for a monitor + audio – rented screens, LED walls, exhibits
A rented LED wall comes with its own processor (a sending card) that takes HDMI / DisplayPort; a screen takes HDMI. The
deck only has to put out 1080p / 4K over HDMI and the sound over HDMI or a jack to the PA.

| Box | Size | Price | Out | Notes |
|---|---|---|---|---|
| **Radxa X4** (Intel N100, 4 / 8 GB, credit-card size) | 85 × 56 mm | ~$65–100 (often out of stock) | 2 × micro-HDMI 4K60, 3.5 mm audio, HDMI audio, 2.5 GbE, Wi-Fi 6 | **x86 – boots our stick today**; the smallest deck that drives a screen with our image; needs its fan / a case with airflow |
| Pocket N100 / N150 box (GMKtec G3 Plus, ZX05) | ~phone size | ~$130–160 | 2 × HDMI, audio | closed case, easy to tape behind a screen |
| Fanless signage stick (MeLE PCG02 Pro) | stick | ~$300–370 | 2 × HDMI 4K | silent, rated 24/7 – for weeks-long exhibits; expensive |
| Raspberry Pi Zero 2 W / Pi 5 | tiny | $15 / $175 | mini- / micro-HDMI | ARM – waits for #174 |

**For exhibits and stages: a Radxa X4 per screen**, an N150 box where it must be enclosed, a signage stick only for long
silent runs.

## Wi-Fi (owner: "Wi-Fi would be nice")
| Device | Wi-Fi | If missing |
|---|---|---|
| T480 / X1 / T14 AMD | yes (Intel AX / AC, iwlwifi) | – |
| N100 / N150 boxes (GMKtec G3 Plus …) | yes, Wi-Fi 6 (often Intel AX201 / Realtek – prefer Intel) | swap the M.2 card for an Intel AX210 (~€15–20) |
| Radxa X4 | yes, Wi-Fi 6 + BT on board | older Debian lacked the driver; our Arch kernel is newer – to check with `deck-check` |
| Steam Deck | yes, Wi-Fi 6 (LCD: Wi-Fi 5) | – |
| T-Watch S3 / phones / tablets | yes | – |
| Raspberry Pi 5 | yes | – |
| HP t640 / Wyse 5070 / Futro | **often without** (sold wired) | the M.2 E-key slot takes an **Intel AX210 + 2 antennas (~€20)**; a USB dongle works, Intel- or MediaTek-based ones are safest (Realtek is hit or miss) |
| RK3588 boards (Orange Pi 5 Plus, Rock 5B) | no, M.2 E-key slot | an M.2 Wi-Fi card |

Wi-Fi for setup and small shows; **a cable for anything on stage that must not drop** (outlets, the conductor) – the
image does both (NetworkManager: wired first, the Wi-Fi picker, the hotspot).

## The GPU per category – where we can step up (owner: "GPU is an obvious limitation")
**What we need today is modest:** the cathedral (96K triangles, 35K after the re-bake) + a being runs 60 fps at 1080p on
an Intel **UHD 620 (~0.4 TFLOPS)**. The GPU becomes the limit for **4K / LED walls, many beings at the conductor, video +
3D composited, and future effects** (the pool's water #196, fluids). **Rule for decks: Intel or AMD** – their Mesa
drivers (ANV, RADV) are open and already in the image; NVIDIA needs the proprietary driver (not in the image, a
maintenance burden) – avoid it for decks, fine for a dev PC. Figures are rough FP32 peaks for comparison only.

| Category | Base (cheap) | Step up | Top (still sane) | Notes |
|---|---|---|---|---|
| Watch | ESP32-S3: **no GPU** (2D sprites) | – | – | faces and cues only |
| Phone / tablet (browser, WebGL2) | budget Mali-G57 class (~0.1–0.2) | mid Adreno 6xx / Mali-G7x | flagship (S22 Xclipse 920 ~1.3, Adreno 7xx) | budget phones: a lower render scale for the being |
| Handheld | Steam Deck LCD, RDNA 2 8 CU (**~1.6**) €299 | Steam Deck OLED (same GPU, better screen) | Ryzen Z1 Extreme handhelds, Radeon 780M (~4.3) ~€500+ | all x86 + AMD: boot the stick |
| Laptop | T480 / X1 Gen 7, UHD 620 (**~0.4**, proven) ~€175 | used **ThinkPad T14 AMD** (Ryzen 5 PRO 4650U / 5650U, Vega 6–7, **~1.1–1.6**) ~€250–350 | Ryzen 6000+ laptops (680M ~3.4) | the AMD ThinkPads are the sweet spot: open RADV, 3–4× the X1 |
| Mini box (screens, LED walls) | N100 / N150, 24 EU (**~0.45**) ~$150 | Ryzen 5 5600U / 5825U box, Vega 7 (**~1.6**) ~€200–280 | Ryzen 7 8845HS box, **Radeon 780M (~4.1–8.6)**: barebone ~$350, configured $520–700+; OCuLink for an external GPU | 780M boxes drive 4K walls and a conductor's crowd of beings |
| Smallest (exhibit puck) | Radxa X4, N100 (**~0.45**) ~$65–100 | – | – | 1080p video + simple 3D; 4K 3D needs a 680M / 780M box |
| SBC (ARM, #174 later) | Pi 5, VideoCore VII (~0.08) | – | RK3588, Mali-G610 (~0.5) | weak for 3D; good as bridges and players |
| Conductor of a big session | T480 for ≤5 decks | T14 AMD | 780M box, or the PC (RTX 2070 ~7.5) | 5 beings cost 1.5 ms CPU / frame today; the GPU sets the crowd size |

## A starter kit per show size (first guess)

| Show | Buy | ~Cost |
|---|---|---|
| A reading with one live screen | 1 used T480 (conductor + outlet in one) | ~€175 |
| The cathedral rehearsal (#190): conductor + 2 AV decks + light | 1 T480 (conductor) + 2 N150 boxes (AV decks on screens) + 1 HP t640 (DMX bridge, #187's Enttec ~€215 extra) | ~€175 + 2 × €150 + €65 ≈ **€540** (+ €215 DMX) |
| A choir piece | the singers' own phones + 1 T480 conductor | ~€175 |

## What we would measure on each (the log in deck/HARDWARE.md)
`deck-check`'s row (GPU, Vulkan, VA-API, audio), the cathedral's fps / CPU per frame at 1080p, cold start, idle RAM,
idle watts, and how many beings a conductor draws at 60 fps. **Buy one T480 and one N150 box first** – they cover the
M tier both ways (all-in-one laptop vs. headless box) – and one t640 for the S tier.

## Sources
- N100 / N150: [raspberry.tips mini-PC comparison 2026](https://raspberry.tips/en/raspberrypi-tutorials/mini-pc-comparison-n100-n150-n350) · [JellyWatch N100/N150 guide](https://jellywatch.app/blog/best-mini-pc-jellyfin-server-n100-n150-n305-2026) · [ASUS NUC 14 Essential review](https://minipc-review.com/en/asus-nuc-14-essential-review) · [eBay.de N150 listing](https://www.ebay.de/itm/157170184633)
- Used ThinkPads: [pcprice.watch used ThinkPad guide 2026](https://www.pcprice.watch/guides/used-thinkpad-buyers-guide-2026) · [Birmingham AV refurb guide](https://www.bavit.co.uk/journal/lenovo-thinkpad-refurb-guide-t14-t480-x1-carbon) · [Notebookcheck X1 Gen 6 deal](https://www.notebookcheck.com/Flaggschiff-Laptop-fuer-239-Euro-Lenovo-ThinkPad-X1-Carbon-Gen-6-im-Refurbished-Deal.934020.0.html)
- Thin clients: [selfhosting.sh refurbished thin clients](https://selfhosting.sh/hardware/refurbished-thin-clients/) · [mydealz HP t640](https://www.mydealz.de/deals/hp-t640-terminal-mini-pc-amd-ryzen-r1505-passiv-aufrustbarer-ram-m2-nvme-slot-usb-c-mit-dp-3xdp-refurbished-dell-wyse-futro-alternative-2421243)
- Raspberry Pi 5: [Tom's Hardware price rises](https://www.tomshardware.com/raspberry-pi/raspberry-pi-5-price-increases-drastically-as-ai-shortage-bites-16gb-version-now-usd205-second-price-increase-in-three-months-over-70-percent-more-expensive-than-original-msrp) · [raspberry.tips price lifecycle](https://raspberry.tips/en/raspberrypi-infos/raspberry-pi-prices-lifecycle) · [Khronos: V3DV Vulkan 1.3](https://www.khronos.org/news/permalink/raspberry-pi-driver-updated-with-vulkan-1.3-support) · [Pi 5 H.264 decode note](https://gist.github.com/schickling/089f4faf412b5267508f758408f0645f)
- RK3588: [raspberry.tips alternatives 2026](https://raspberry.tips/en/raspberrypi-tutorials/raspberry-pi-alternatives-2026-orange-pi-5-rock-5b-odroid-n2-compared) · [ChipRadar Pi 5 alternatives](https://chipradar.io/blog/raspberry-pi-5-alternatives-2026) · [Collabora: PanVK Vulkan 1.1 conformant](https://www.collabora.com/news-and-blog/news-and-events/panvk-is-officially-vulkan-11-conformant.html) · [Phoronix: PanVK Vulkan 1.2](https://www.phoronix.com/news/PanVK-Vulkan-1.2-Merged-Mesa)
- Screens, smallest box, GPU: [XDA Radxa X4](https://www.xda-developers.com/radxa-x4-review/) · [Tom's Hardware Radxa X4](https://www.tomshardware.com/raspberry-pi/radxa-x4-review) · [sbc.compare Radxa X4](https://sbc.compare/radxa-x4) · [LILYGO T-Watch S3 Plus](https://lilygo.cc/products/t-watch-s3-plus) · [LinuxGizmos T-Watch S3](https://linuxgizmos.com/low-cost-smartwatch-powered-by-esp32-s3-module/) · [Notebookcheck Steam Deck refurbished](https://www.notebookcheck.com/Steam-Deck-gibts-jetzt-ab-299EUR-generalueberholt-direkt-bei-Valve.1308058.0.html) · [MonsterDealz Steam Deck 2026](https://www.monsterdealz.de/magazin/steam-deck-lohnt-sich-2026) · [CNX ZX05](https://www.cnx-software.com/?p=119058) · [MeLE PCG02 Pro](https://pricehistory.app/p/mele-pcg02-pro-fanless-mini-pc-stick-buP9XC2h) · [Minisforum UM880 Plus](https://store.minisforum.com/products/minisforum-um880-plus-mini-pc) · [XDA GMKtec K8 Plus deal](https://www.xda-developers.com/gmktec-gaming-mini-pc-k8-plus-689-deal/) · [camelcamelcamel K8 Plus](https://camelcamelcamel.com/product/B0DHNTW3H6)
