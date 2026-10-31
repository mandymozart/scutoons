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
