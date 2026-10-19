Status 2026-10-19: research + built in the storyboard studio (worktree branch, not merged) – the owner's S22 test is open (§7).

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
| **USB + Chrome port forwarding** (`chrome://inspect` → Port forwarding 14000 → localhost:14000) | USB debugging on once | The phone opens `http://localhost:14000/phone` – localhost is a secure context, no certificate, the studio stays on 127.0.0.1. **Recommended.** (`adb reverse tcp:14000 tcp:14000` does the same.) |
| Phone flag `chrome://flags/#unsafely-treat-insecure-origin-as-secure` = `http://<pc-ip>:14000` | 1 min | Wi-Fi, no cable; the studio must listen on the LAN (`STUDIO_LAN=1`). |
| Self-signed / mkcert certificate (`STUDIO_HTTPS=1`, files in `workflows/storyboard-studio/.cert/`) | 10 min | Wi-Fi; a self-signed one needs "Proceed anyway" once; mkcert's CA on Android is fiddly. |
| The live studio URL | – | Doesn't exist: the studio is a local tool (its API writes the repo's files). |

On the LAN the studio's API could write files and run scripts – so in LAN mode requests from other machines get
only GETs (the page, the render files) and the WebSocket; every write stays on the PC.

## 4. Transport

- **WebSocket relay through the studio's own dev server** (Vite's HMR socket, custom events `hh`): the phone sends its
  pose, the dev server forwards it to the PC page of the same pairing code. The relay runs on the same PC as the
  receiving page, so it is one LAN hop (USB: < 5 ms, Wi-Fi: 5–20 ms). No dependency, no extra port.
- **WebRTC data channel**: peer to peer, unreliable/unordered mode avoids head-of-line blocking – the right choice
  when phone and viewer are on different networks. Here both ends meet at the PC anyway; it would add signalling
  (which again needs the socket) for no gain. Not now.
- Samples carry the phone's own timestamps, so jitter in transport never shows in the recorded track.

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

**WebXR `immersive-ar` + `dom-overlay` in Chrome on the S22, USB port forwarding to `localhost`, the studio's own
WebSocket as relay, raw samples + one exponential filter, passes beauty / depth / normals / grey rendered in the
studio's browser with its own renderer and encoded by ffmpeg.** DeviceOrientation + drag as the fallback (and for
tests on the desktop). Built: `workflows/storyboard-studio/README.md` "Handheld camera". Not built: FBX (convert the GLB
in Blender: File → Import → glTF 2.0, then File → Export → FBX), animated lens (glTF has no animated `yfov` without
KHR_animation_pointer – the lens is constant per take; the JSON carries `fov` per sample), WebRTC, zoom on the phone.

The owner's test with the real S22: the steps are in the studio README "Handheld camera – first test with the phone".
