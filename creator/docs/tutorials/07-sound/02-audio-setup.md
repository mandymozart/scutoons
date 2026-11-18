---
slug: audio-setup
title: Set up your sound card
summary: Driver, device, buffer, channels and test tones
who: performer, sound
where: editor, deck
often: 1
---

## Facts

- On a deck: deck --audio cpal --audio-host asio --audio-device fireface --audio-buffer 128 (--audio-list shows the devices).

## Steps

1. The menu (top left) → “Audio & inputs” opens the “Audio” panel.
2. Under “HOST” choose the driver: ASIO for an RME and other studio cards, PipeWire on a deck, WASAPI otherwise. Then pick the device.
3. Under “BUFFER” choose a size; smaller is quicker but harder for the computer. With ASIO the size is set in the driver’s own dialog.
4. “GROUPS ON CHANNELS”: click a cell to put a group of the rig’s outlets on a channel. A channel number or a group’s name plays a test tone; click again to stop it.
5. “Apply” writes the settings and restarts the stage’s sound on them.

## Note

Under WASAPI the buffer only grows (an RME gets about 1056 frames): for small buffers use ASIO.
