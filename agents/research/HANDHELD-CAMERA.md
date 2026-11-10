Status: built and in the creator (4003fa79, the phone at /h/<code>); since #469 (2026-11-10) on the live road – one relay session, QUIC first, the WebSocket last, the code claimed once for a handheld token (§4); the WebRTC road the owner tested on the S22 (2026-10-31) is gone, the S22 test of the live road is the owner's. Reused for the deck camera (#213).

# HANDHELD-CAMERA — the phone as a handheld camera for storyboard shots (research, 2026-10-19)

Owner 2026-10-19: "Is there a simple way to use my Samsung S22 for that, in the browser? In the storyboard module I'd
enable it per shot. I connect my phone; it shows me the scene fullscreen; on the PC the camera of that shot in the shot
list moves along with the phone and records the camera movement. Afterwards I only need to trim the camera movement,
and then a function that renders all shots made this way as videos, which I then send to an AI as control frames."
Later the same day: also as downloads (GLB with the animated camera, a JSON of the samples; per shot and as a zip).

## 1. How the popular workflows do it

- **The trend** ("block out in Blender, film it with the phone, render with AI"): a grey block-out scene, a phone app
  streams or records its 6DoF pose (ARKit / ARCore visual-inertial tracking), the pose drives the 3D camera, the take is
  rendered as grey / depth / lineart passes and fed to a video model as a control video (Runway Gen-4 / Aleph video to
  video, Luma Modify Video, Wan 2.1/2.2 VACE or Fun-Control in ComfyUI, Kling with a reference video).
- **Phone-as-virtual-camera tools:** VirtuCamera (Blender add-on + app), Unreal's Live Link VCAM (iOS only), Lightcraft
  Jetset, CamTrackAR (records the track, exports FBX / Blender script), Blender add-ons that read a recorded ARKit/ARCore
  CSV. All native apps: tracking from the AR framework, transport over Wi-Fi (OSC / WebSocket / Live Link), recording on
  the desktop side, export as an animated camera.
- **In the browser** the same tracking exists: Chrome on Android exposes ARCore through **WebXR** (`immersive-ar`),
  i.e. the full 6DoF pose of the phone every frame. Nothing to install on the phone except Chrome + "Google Play
  Services for AR" (preinstalled on the S22).

## 2. What the S22 supports

- **WebXR `immersive-ar` with ARCore** (Chrome 81+; the S22 is ARCore-certified): `XRFrame.getViewerPose(local)` =
  position in metres + orientation, gravity-aligned (Y up), ~60 Hz, drift-corrected by the visual tracking. Feature
  `dom-overlay` (Chrome 83+) lets a page draw its own fullscreen DOM / canvas over the AR view – so the phone can show
  the shot's own render (the Sin City pass, the shot's lens) instead of the camera image. Needs light and some texture
  in the room; tracking is lost when the lens is covered.
- **Fallback `DeviceOrientationEvent`** (every phone): rotation only (alpha / beta / gamma → quaternion), alpha drifts
  and is relative; no position – the studio adds it by dragging (one finger = move, two fingers = up / down).
- Both need a **secure context** (HTTPS or `localhost`).

## 3. HTTPS for a phone on the LAN

| Way | Effort | Note |
|---|---|---|
| **Self-signed certificate on a LAN port** (built 2026-10-19, owner: "i don t want to plug in a cable") | none – "Continue" once per certificate | **The default.** `https://<pc-ip>:14443` (phone-https.mjs: openssl makes the certificate for localhost + the LAN addresses into `.cert/`; GET / HEAD + the socket proxied to the studio on 127.0.0.1, writes refused); the PC stays on `http://localhost:14000`. |
| USB + Chrome port forwarding (`chrome://inspect` → Port forwarding 14000 → localhost:14000) | USB debugging on once | No Wi-Fi: the phone opens `http://localhost:14000/phone` – localhost is a secure context, no certificate. (`adb reverse tcp:14000 tcp:14000` does the same.) |
| Phone flag `chrome://flags/#unsafely-treat-insecure-origin-as-secure` | 1 min | Not taken (a flag per phone; the studio no longer listens on the LAN over http). |
| mkcert's CA on the phone | 10 min | Not taken (fiddly on Android); the self-signed one needs only "Continue". |
| The live studio URL | – | Doesn't exist: the studio is a local tool (its API writes the repo's files). |

On the LAN the studio's API could write files and run scripts – so in LAN mode requests from other machines get
only GETs (the page, the render files) and the WebSocket; every write stays on the PC.

## 4. Transport (how it is now: the live road, #469)

Owner 2026-11-10: *"We have to update our communication for the handheld to the new dogma. Real time is key. So quic and
fallback rules apply"*. The phone and the PC are two clients of one relay session (research/RELAY-TRANSPORTS.md,
research/INPUT.md §6a "The road"):

- **The right to join.** The Handheld panel's `POST /storyboard/pairings` makes a one-time code (the QR to `/h/<code>`,
  10 minutes) and a fresh relay session (`H` + 11 characters) and gives the PC a `conductor` token for it. The phone's
  page claims the code (`POST …/{code}/claim`, no sign-in, ONE claim, then `409 claimed`) and gets the session and a
  `handheld` token for it. Both are the relay's HMAC tokens (`LIVE_TOKEN_SECRET`; `dev` on a dev relay), 12 hours long;
  the relay refuses a `handheld` without one (`check_token`), and a token names one session and one role. The code opens
  nothing else of the API (server/docs/API.md "Handheld pairing", Domain\Storyboard\Pairings).
- **The one client.** Both pages join through `@scutoons/live-deck/hand` (`HandLink`) on deck-space's relay client
  (`deck/space/src/relay.rs`, relay-web's wasm, the creator's `/live/`), handed in by the page's Svelte side
  (`creator/src/lib/handheld/hand-link.ts`, `hand-port.ts`). The core picks the socket: QUIC gets 250 ms, then the
  WebSocket is dialled beside it; QUIC dying resumes on the WebSocket as the same client; from the WebSocket it tries QUIC
  again after 60 s. No ladder logic in TS.
- **The lanes.** `hand.pose` (`t` the phone's clock in ms, `p` 3, `q` 4 numbers) is latest-wins: on QUIC a datagram,
  never logged or replayed; the relay and the editor's core drop a pose that is malformed or not newer than that phone's
  last. `hand.state` (mode `xr` · `orientation` · `none`, tracking, browser), `hand.record`, `hand.recenter` (phone →
  the session's conductor) and `hand.view` (editor → its phones: shot, on, recording, seconds) are reliable. The core
  sends the phone's state and the editor's view again after every rejoin, the view also to a phone that joins later, and
  tells the editor when a phone left (the relay's presence lacks it; it may come back as the same client).
- **The rung.** The phone's top bar and the panel's state line end in the core's word for the rung: `QUIC` or
  `WebSocket`.
- **Where the relay is:** the creator's `VITE_LIVE_URL` (default `wss://live.scutoons.com`, WebSocket only until its UDP
  endpoint exists, #441) and `VITE_LIVE_QUIC`. QUIC needs a relay with a certificate near the phone (a LAN relay under
  `*.lan.scutoons.com`); a browser without WebTransport goes to the WebSocket at once. The phone no longer has to be in
  the PC's Wi-Fi.
- Samples carry the phone's own timestamps, so jitter in transport never shows in the recorded track.
- **Not measured** (the owner's, afterwards): the pose's delay phone → relay → PC on QUIC against the old WebRTC road, and
  WebTransport on the S22 inside an AR session.

## 5. Smoothing

- Raw ARCore pose already looks like a real handheld camera (micro shake, footsteps) – the owner's "raw handheld look"
  = smoothing 0.
- One filter, the same live and offline: exponential smoothing per sample, `a = 1 − exp(−dt / τ)`, position lerp +
  quaternion slerp, `τ = smoothing × 0.5 s` (0 … 1 → raw … a slow steadicam). The take stores the RAW samples + its
  smoothing value; render and export bake the smoothed track, so the value can change after recording.
- Heavier options (one-euro filter, spline fit) only if the owner asks.

## 6. What the AI tools want as control video

- **Depth** (white = near, black = far, normalised per take – a stable range across frames, no flicker): Wan VACE /
  Fun-Control, ControlNet depth, Runway / Luma as a structure reference.
- **Canny / lineart**: the tools compute them from any clean render; the studio's beauty pass already IS lineart +
  hatch (Sin City look).
- **Normals** (view-space RGB): ControlNet normal; cheap (one more pass).
- **Plain grey render** (lit, no style): what the Blender block-out videos send to Runway / Luma "modify video".
- Format: MP4 (H.264, yuv420p, 24 fps) everywhere; WebM for the browser. Short clips (≤ 10 s), the aspect of the shot.

## 7. Recommendation (built)

**WebXR `immersive-ar` + `dom-overlay` in Chrome on the S22, Wi-Fi to the studio's HTTPS LAN port (2026-10-19; USB port forwarding to `localhost` without Wi-Fi), the studio's own
WebSocket as relay, raw samples + one exponential filter, passes beauty / depth / normals / grey rendered in the
studio's browser with its own renderer and encoded by ffmpeg.** DeviceOrientation + drag as the fallback (and for
tests on the desktop). Built: `workflows/storyboard-studio/README.md` "Handheld camera". Not built: FBX (convert the GLB
in Blender: File → Import → glTF 2.0, then File → Export → FBX), animated lens (glTF has no animated `yfov` without
KHR_animation_pointer – the lens is constant per take; the JSON carries `fov` per sample), zoom on the phone. The
transport of the creator's version is the live road (§4, #469).

The owner's test with the real S22: the steps are in the studio README "Handheld camera – first test with the phone".
