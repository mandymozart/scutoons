# Storyboard Studio

A local tool for the storyboard workflow (owner 2026-10-09: "just start the studio over night. i want to see where
this goes. we might dump it later anyhow if there is easier way"; 2026-10-10: "use our scutoons ui and design
dogmas", "a free moving camera in a 3d view"). Built like the tools: `@scutoons/ui` (AppShell, dock panels, rails,
Segmented, dialogs) and `@scutoons/tokens` (tools.css). Everything goes back into the repo's storyboard files.

```bash
npm run dev:studio
```

→ http://localhost:14000 (`shared/dev-ports.json` "studio"; `npm run dev -- studio` with the stack). A Vite + Svelte app
(no SvelteKit); its API (`api.mjs`) runs inside the dev server. Needs the local dev DB for the script text
(`npm run dev:db`) and the media dev copy (`media/`) for the character images.

## The layout

- **Sidebar – the shots** of the chapter in reading order, by scene: frame, id, format, view, review dot, variants.
  Resizable, folds to a rail.
- **Page – the 3D view**: the shot's scene with the frames' own renderer (`tools/storyboard-render/core.js`), live.
  Drag = orbit, right-drag = pan, wheel = dolly – the camera is written into the shot as a hand camera
  (`camera: { pos, target, fov }`); "Shot camera" goes back to the camera of size + angle. A figure is handled
  directly: drag it = move on the ground (Turn tool: drag sideways = turn), wheel on it = size, Alt + wheel = turn (about
  its feet); the picked figure's gizmo works too. Its place goes back once per gesture as the storyboard's x / y / h /
  turn – the exact inverse of the renderer's placing, so a rebuild (any field changed) leaves every figure where it was
  (a figure floating under a hand camera gets `ground: false`). The studio's text is 60 % of the tools' root. The texts
  in the SCUTOONS presets on top (switchable). **Look switches**: shadows Solid (Sin City, the default) · Hatch
  (crosshatching) · Tone, inner lines, colour, wobble (after Maxime Heckel's "Moebius-style post-processing"; black
  and white unless colour is on).
- **Right dock** (toggles in the top bar; each folds to a rail, drags wider): **Shot** – every field (what it shows,
  format + camera, light incl. a free direction, figures) + Save + frame / Keep as variant / Revert / Approve / Ask for a
  change / Rule (→ storyboard-taste.md); **Script** – the scene's script lines + notes; **Variants** – the saved frame,
  variants (Load / Take this), generated images dropped in (Approve as chosen → the dataset); **Luma** – the full prompt
  from the shot as set (scripts/storyboard/luma.js), the render without texts, the references to attach; **Handheld** –
  below; **Generate** – a generator per shot (below, "Generators").
- **Top bar**: chapter, summary, Check, All frames, Reference packs, Chapter draft.

## Files

- the shots: `storyboards/<toon>/<chapter>/scene-*.json` (written as 2-space JSON)
- variants: `storyboards/<toon>/<chapter>/variants/<shot>.json`, frames under `storyboards/_out/<chapter>/` (`frames/`,
  `clean/` without texts, `variants/`, `generated/` + `generated.jsonl` – the generators' results and their log)
- the generators: `generators/` (`jobs.mjs` the interface, `runway.mjs`, `comfy-local.mjs`, `others.mjs`; tests:
  `npm test -w workflows/storyboard-studio`)

## Handheld camera (the phone as the shot's camera)

Owner 2026-10-19: film a shot's blocked scene with the phone, trim, render control videos for AI, download the camera
(research/HANDHELD-CAMERA.md: why WebXR, USB, the socket relay, the passes).

- **Per shot**: the **Handheld** panel → "Handheld camera for <shot>" (stored in the shot's takes file, below).
- **Pairing**: "Connect a phone" shows a 4-digit code, a QR code and the phone link `/phone?code=1234` (a route of this
  app). The phone page shows the shot fullscreen through the shot camera (the studio look; "Plain" = a faster plain
  render). **Start (AR)** = WebXR `immersive-ar` (ARCore: position + rotation, Chrome on Android); **Start / Rotation
  only** = DeviceOrientation (rotation) + drag for position (one finger = move, two = up / down). The first pose starts
  at the shot camera; **Set origin** (phone or PC) = the phone's pose now is the shot camera's pose (heading matched,
  up stays up). The PC's 3D view follows live; **Smoothing** 0 = the raw handheld look … 1 = a slow steadicam;
  **Scale** = scene metres per phone metre (a miniature set: < 1).
- **Record / Stop** on the PC or the phone → a take: samples `{ t, position, quaternion, fov }` (~60 Hz, the scene's
  metres, Y up, glTF's camera convention) in `storyboards/<toon>/<chapter>/takes/<shot>.json` (`{ on, active, takes }`;
  raw samples + the take's smoothing, in / out). Several takes per shot, one active (the radio); delete = the bin.
- **Trim**: the timeline – drag the in / out handles (or ← → on them, Shift = 1 s), drag / click the bar = scrub, Play
  = the kept part; the 3D view shows the moment. "Shot camera" gives the view back to the orbit.
- **Render all handheld shots** (the panel's bottom): every shot with the camera on and an active take, along its
  trimmed, smoothed track, rendered in the studio's browser with the frames' renderer, at 24 / 25 / 30 fps, 1× or 2×.
  Passes: **Beauty** (the studio look – lines + hatch, also the lineart), **Depth** (white = near, inverse depth with
  one range per take), **Normals** (view space), **Grey** (plain lit). ffmpeg (on the PATH, or `FFMPEG=<path>`) writes
  `storyboards/_out/<chapter>/handheld/<shot>--<take>--<pass>.mp4` (H.264, yuv420p; + `.webm` VP9 when ticked) – the
  links appear under the button. These are the control videos for Runway / Luma / Wan VACE / ControlNet.
- **Downloads**: per take "Camera as glTF" (`<shot>-<take>-camera.glb`: a perspective camera with its animation;
  "with the scene" adds the blocked set + figures) and "JSON" (the trimmed, smoothed samples + the space it is in);
  "Download all (zip)" = both for every handheld shot + a README.
  - **Blender**: File → Import → glTF 2.0 (.glb) – the camera comes with its action (Y up becomes Z up by itself).
    FBX: import the GLB in Blender, then File → Export → FBX.
  - **Unreal**: import the GLB (the glTF / Interchange importer), put the camera into a Level Sequence (Sequencer) –
    or import Blender's FBX as a camera track in Sequencer.
  - The lens is constant per take in the GLB (glTF animates no `yfov` without KHR_animation_pointer); the JSON has
    `fov` per sample.
- A take lives in the scene's world: the figures are placed through the shot camera, so changing the shot's size /
  angle / hand camera after recording moves them – record again.

### Connecting the phone

- **USB (recommended)**: phone → Developer options → USB debugging on; cable to the PC; in desktop Chrome
  `chrome://inspect/#devices` → **Port forwarding…** → `14000` → `localhost:14000`, tick "Enable port forwarding". The
  phone opens `http://localhost:14000/phone?code=…` – a secure context (WebXR / sensors need one), no certificate,
  the studio stays on 127.0.0.1. (`adb reverse tcp:14000 tcp:14000` does the same.)
- **Wi-Fi**: `STUDIO_LAN=1 npm run dev:studio` listens on the LAN (other machines get only GETs and the socket – every
  write stays on this PC; only in a trusted network). The phone needs a secure context: either its Chrome flag
  `chrome://flags/#unsafely-treat-insecure-origin-as-secure` = `http://<pc-ip>:14000`, or HTTPS with
  `STUDIO_HTTPS=1` and `workflows/storyboard-studio/.cert/key.pem` + `cert.pem` (gitignored; mkcert or
  `openssl req -x509 -newkey rsa:2048 -nodes -days 365 -subj /CN=studio -keyout key.pem -out cert.pem`; the phone
  accepts the warning once).
- Transport: the studio's own dev-server socket (`handheld-relay.mjs`, Vite's HMR WebSocket) – no extra port.

## Generators

Owner 2026-10-19: "Ja, generator pluggable ist smart. Erstmal soweit bauen, dass ich die Runway API anbinden kann."
The **Generate** panel sends a shot to a generator; the result lands in Variants → Generated
(`_out/<chapter>/generated/<shot>--g<n>.png|mp4`, logged in `generated.jsonl` with generator, model, estimate, cost
and the provider's task id), so "Approve as chosen" works as before. Each generator says what it takes; the inputs are
pre-filled from the shot (the approved image, else the render without texts; the newest handheld render pass; the
shot's "Image (EN)" text as the prompt).

| Generator | What | Cost |
|---|---|---|
| ComfyUI (this PC) – `comfy-local` | `scripts/storyboard/comfy.mjs` unchanged: Qwen-Image-Edit, the render + character images → a still | free |
| Runway – `runway` | **Gen-4 Turbo** / **Gen-4.5** image to video (the still + prompt, 2–10 s); **Aleph 2** video to video (the handheld render pass ≤ 30 s, restyled by the prompt; the still as keyframe) | 5 / 12 / 28 credits per second (Aleph at least 56); 1 credit = 0.01 USD |
| ComfyUI (cloud GPU) – `comfyui-remote` | the same graphs on RunPod / Comfy Cloud – not built yet (`generators/others.mjs`) | – |

Safety: the estimate is shown before sending; every paid call asks once more with the cost; nothing is retried by
itself (a failed or timed-out create call is reported, not sent again); the panel shows what this studio session has
spent (it resets when the studio restarts). Cancel stops the job and cancels the task at Runway.

### Runway einrichten (für den Owner)

1. Auf **https://dev.runwayml.com** anmelden (das Entwicklerportal – nicht das normale Runway-Abo, die Credits sind
   getrennt), ein Projekt anlegen, Credits kaufen (1 Credit = 0,01 USD). Der Start-Tier erlaubt bis 100 USD im Monat
   und wenige Generierungen gleichzeitig / pro Tag; mehr gibt es automatisch nach den ersten Käufen.
2. Unter **API Keys** einen Schlüssel erzeugen und kopieren (er wird nur einmal angezeigt).
3. In die Datei **`.env` im Repo-Ordner** (`C:\Development\scutoons\.env`, sie ist in `.gitignore`, wird nie
   committet) eine Zeile schreiben: `RUNWAY_API_KEY=key_…`. Ein Neustart des Studios ist nicht nötig.
4. Im Studio: Shot wählen → **Generate** → Generator „Runway“, Modell wählen, Bild / Video und Prompt prüfen →
   **Generate** → die Rückfrage zeigt den Preis → bestätigen. Das Video erscheint unter Variants → Generated.

Was ein Shot kostet (Preise laut docs.dev.runwayml.com, Stand 2026-10-19):

| Modell | pro Sekunde | 5-s-Shot | 10-s-Shot |
|---|---|---|---|
| Gen-4 Turbo (Bild → Video) | 5 Credits = 0,05 USD | 0,25 USD | 0,50 USD |
| Gen-4.5 (Bild → Video) | 12 Credits = 0,12 USD | 0,60 USD | 1,20 USD |
| Aleph 2 (Handheld-Video → Video) | 28 Credits = 0,28 USD (min. 0,56 USD) | 1,40 USD | 2,80 USD |

Ohne Schlüssel zeigt der Generator „not configured“ mit dieser Anleitung in einer Zeile. Der Schlüssel bleibt auf dem
PC: er geht nie an den Browser und nie in Git.

For development: `STUDIO_FAKE_GENERATOR=1` adds a fake generator (fake credits); `RUNWAY_API_URL` points the adapter at
the mock (`node workflows/storyboard-studio/generators/test/runway-mock.mjs --port 14091`, key `mock-key`).

## Not yet

- joints posed by hand in 3D (poses come from the pose list), props placed by hand, undo;
- the look saved per chapter (the switches are a view setting; `render-shots.mjs --look hatch,inner` for files);
- the move into the creator, if the team works with it (agents/STORYBOARD.md §7 / phase 5).
