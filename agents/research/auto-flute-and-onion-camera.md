# Research: Auto Flute and Onion camera

Date: 2026-10-02. Status: research only, nothing built. Context: the SvelteKit viewer on phone browsers (Android
Chrome / Samsung Internet, iOS Safari). We already lazy-load TF.js 4.22 and `@tensorflow-models/mobilenet` 2.1.1
(v2, alpha 1.0) for `scan_object` (`viewer/src/lib/components/camera/object-detector.ts`, `classify()` + `embed()`).

Versions and licences come from the npm registry and the GitHub API, checked on 2026-10-02. Numbers marked
**(claimed)** are the authors' own figures. Numbers marked **(estimate)** are my judgement and must be measured on our
own phones. I took nothing on a phone myself.

---

## Part A: Auto Flute (sing, and it plays theremin-like harmonies)

### TL;DR

- **Pitch:** use the **McLeod Pitch Method via `pitchy`**: about 33 KB, MIT/0BSD, under 1 ms per frame. Run it in an
  AudioWorklet, or for a first prototype on an AnalyserNode. Don't ship an ML pitch model in v1. CREPE-tiny (about
  2 MB) and SPICE are options for later, if real-world noise turns out to be a problem.
- **Harmony:** use **rule-based music theory**: smooth the pitch, estimate the key from recent notes, add a diatonic
  third or sixth, and optionally a triad pad. The ML options (Coconet, MusicRNN, DDSP) are offline or phrase-level
  only, and Magenta.js pins TF.js 2.7, which clashes with our TF.js 4.22. They could at most power a later
  "replay my phrase as a Bach chorale / as a real flute" mode.
- **Sound:** use **plain Web Audio** with no dependency: oscillators with `setTargetAtTime` glides (portamento),
  breath noise, vibrato and a generated-impulse ConvolverNode reverb. Tone.js 15 (MIT) is an optional convenience.
  A Faust physical flute model (precompiled WASM) can be an upgrade later.
- **Biggest risk: feedback.** The phone speaker plays a clean tone a third above the voice, the microphone hears it,
  and the detector locks onto it. Plan for headphones, voice gating, rejecting our own frequencies, and the iOS
  audio-session quirks from the start.

### A1. Pitch detection in the browser

| Option | What it is | Size | Licence | Latency / cost | Fit |
|---|---|---|---|---|---|
| [`pitchy`](https://github.com/ianprime0509/pitchy) 4.1.0 | McLeod Pitch Method (MPM, McLeod & Wyvill, "A Smarter Way to Find Pitch", ICMC 2005); returns `[hz, clarity 0..1]` | 33 KB unpacked, no deps, ESM | MIT on npm 4.1.0 (repo now 0BSD) | Window 1024–2048 samples = 21–43 ms at 48 kHz; compute is tiny (FFT autocorrelation) **(estimate: <1 ms/frame)** | **Recommended v1.** Made for tuners. `clarity` doubles as the voiced/unvoiced gate |
| [`pitchfinder`](https://www.npmjs.com/package/pitchfinder) 2.3.4 | YIN, AMDF, MacLeod, … in JS | 229 KB | **GPL-3.0**, avoid | similar | Licence is a problem for our closed code |
| [`aubiojs`](https://github.com/qiuxiang/aubiojs) 0.2.1 | aubio compiled to WASM (yinfft by default, 8 methods) | 427 KB | Wrapper MIT, but **aubio itself is GPL-3.0** | fast | Same licence problem; last release 2023 |
| CREPE "tiny" via [ml5 0.12](https://github.com/ml5js/ml5-library) | CNN pitch estimator ([marl/crepe](https://github.com/marl/crepe), MIT, ICASSP 2018) | about 1.96 MB of weights (sum of the shards in [ml5-data-and-models/…/crepe](https://github.com/ml5js/ml5-data-and-models/tree/main/models/pitch-detection/crepe)) | MIT | 16 kHz input, 1024-sample frame (64 ms), 10 ms hop; one CNN pass per hop on WebGL | Robust to noise, but heavier. **ml5 1.x dropped `pitchDetection`**, and ml5 0.12.2 pins TF.js 1.7, so we would port the model to our TF.js 4 ourselves |
| SPICE ([paper](https://arxiv.org/abs/1910.11664), [TF blog](https://blog.tensorflow.org/2020/06/estimating-pitch-with-spice-and-tensorflow-hub.html)) | Self-supervised pitch CNN; TF.js build on TF Hub, now [Kaggle Models](https://www.kaggle.com/models/google/spice) | not stated | Apache-2.0 (TF Hub models) | 16 kHz mono; returns pitch and uncertainty; works with background noise **(claimed)** | Used by FreddieMeter and Tone Transfer. Also wrapped in `@magenta/music` (TF.js 2.7) |
| [Spotify Basic Pitch](https://github.com/spotify/basic-pitch-ts) (`@spotify/basic-pitch` 1.0.1) | Polyphonic audio→MIDI with pitch bends | model about 0.9 MB (model.json 175 KB + 742 KB weights), <17k params, <20 MB peak memory **(claimed)** | Apache-2.0 | Works on whole windows of about 2 s, so it's a transcriber, not a live tracker; pins TF.js ^3.2 | **Not for live use.** Useful only if we ever want "turn my humming into notes" offline |
| [PESTO](https://github.com/SonyCSLParis/pesto) ([paper 2025](https://arxiv.org/abs/2508.01488)) | Streaming self-supervised pitch, 130k params, <10 ms latency **(claimed)** | tiny | **LGPL-3.0**, PyTorch, no web build | real-time | Interesting state of the art, but we'd need our own ONNX export plus ORT-Web |

**AudioWorklet vs ScriptProcessor vs AnalyserNode**

- `ScriptProcessorNode` is deprecated and runs on the main thread. Don't use it.
- **AudioWorklet** is supported in Chrome 66+ and Safari 14.1+. It gets 128-frame render quanta on the audio thread.
  Keep a ring buffer of 2048 samples, run MPM every 256–512 samples (5–10 ms hop), and `postMessage` the
  `{hz, clarity, rms}` to the main thread. `pitchy` is pure JS, so it can be imported into the worklet module.
- For the quickest prototype: `AnalyserNode.getFloatTimeDomainData()` in a `requestAnimationFrame` loop (about
  60 Hz). This is good enough to feel out the instrument, though it adds jitter and pauses in a background tab.

**Sample rate**

- Phones run at 48 kHz (some Android devices at 44.1 kHz). Don't force a rate. Read `ctx.sampleRate` and pass it to
  `findPitch`.
- The singing range is about 80–1000 Hz. MPM needs about 2 periods (25 ms at 80 Hz), so a 2048 window at 48 kHz is
  safe and 1024 suffices above about 100 Hz.
- Disable `autoGainControl` and `noiseSuppression` in the getUserMedia constraints, because they smear pitch.
  `echoCancellation` is discussed under A4.

**Post-processing (this matters more than the algorithm)**

- Gate on `clarity > ~0.9` and RMS above a noise floor.
- Run a median filter over 3–5 frames to kill octave jumps.
- Hold the last pitch for about 80 ms across short dropouts.
- Convert to MIDI as a float: `69 + 12·log2(f/440)`.

### A2. Harmony generation

**Rule-based (recommended, real-time, deterministic, tiny)**

1. **Note tracking:** take the smoothed float MIDI, and call a "note" stable when it stays within ±40 cents for
   ≥120 ms.
2. **Key estimation:** keep a decaying pitch-class histogram (duration-weighted, half-life about 8 s). Correlate it
   with the Krumhansl–Schmuckler major/minor profiles (24 keys) and switch keys only with hysteresis.
   - For v1, consider a **fixed key per item or per moment**, e.g. the flute is "in C". Snapping the voice to a known
     scale is how [Blob Opera](https://experiments.withgoogle.com/blob-opera) avoids wrong notes: "pitches stick to a
     scale, so there are no wrong notes".
3. **Harmony voice:** snap the sung pitch to the nearest scale degree, then take the **diatonic third above** (a
   major or minor third depending on the degree). Offer a **sixth below** as an alternative register for high
   voices.
   - Glide the harmony continuously: let it follow the raw sung pitch's micro-movements (vibrato, slides) as an
     offset from the snapped target, so it feels like a theremin rather than a keyboard.
4. **Chord pad (optional):** a triad on the current scale degree that changes only when a note is stable for
   ≥300 ms. Use a simple functional preference (I, IV, V, vi) when the melody note belongs to several chords.

All of this is a few hundred lines of TypeScript and testable in `@scutoons/content`-style unit tests.

**ML options and what they can really do**

| Model | What it does | Runs in real time? | Notes |
|---|---|---|---|
| **Coconet** ([Magenta](https://magenta.tensorflow.org/coconet), [Bach Doodle paper](https://arxiv.org/abs/1907.06637)) | Fills in 4-part Bach-style counterpoint for a given melody | **No, it's phrase-level.** For the Doodle, Google cut runtime from 40 s to **about 2 s** per harmonisation and the download to **about 400 KB** (quantised) **(claimed)**, and sent slow devices to TPUs | Good for a "replay your phrase as a chorale" feature, not live |
| **MusicRNN / ImprovRNN / MusicVAE** (`@magenta/music` 1.23.1, Apache-2.0) | Melody continuation, chord-conditioned improvisation, interpolation | Generation runs per step or sequence, not as an audio-rate harmoniser | Built for MIDI sequences, not a gliding voice |
| **Piano Genie** ([paper](https://arxiv.org/abs/1810.05246), [demo](https://magenta.github.io/magenta-js/music/demos/piano_genie.html)) | 8 buttons → 88-key piano, LSTM 2×128 | **Yes**, real time in the browser | A nice concept (constrained input → musical output), but the wrong input type |
| **DDSP / Tone Transfer** ([Tone Transfer](https://magenta.tensorflow.org/tone-transfer), [about](https://sites.research.google/tonetransfer/about), [DDSP-VST](https://magenta.withgoogle.com/ddsp-vst-blog)) | Re-renders a voice as flute, sax, trumpet or violin | **No, not in the browser.** The magenta.js demo ([`ddsp_tone_transfer.ts`](https://github.com/magenta/magenta-js/blob/master/music/demos/ddsp_tone_transfer.ts)) runs SPICE on the whole buffer, then `ddsp.synthesize(features)` on the whole buffer. DDSP-VST is real time, but it's a native plugin and was archived in 2024 | The flute checkpoint is hosted under `storage.googleapis.com/magentadata/js/checkpoints/ddsp/flute`. Offline "hear yourself as a real flute" is a fun later mode |
| "AI Harmonizer" ([NIME 2025, arXiv 2506.18143](https://arxiv.org/abs/2506.18143)) | Solo voice → 4-part harmony | **Offline only** (the authors list real time as future work) | Confirms there's no off-the-shelf real-time ML harmoniser |
| Blob Opera | A real-time ML harmoniser for 4 voices | Yes, but closed source | Proof the UX works; see A5 |

**Practical blocker:** `@magenta/music` 1.23.1 depends on **`@tensorflow/tfjs ^2.7.0`** and `tone ^14.7`. Mixing it
with our TF.js 4.22 means two TF.js runtimes (several MB more, two WebGL contexts). Any Magenta feature should either
port the single checkpoint to TF.js 4 or live in its own lazy chunk. The main `magenta/magenta` repo was archived in
January 2026. `magenta-js` is still maintained.

### A3. Synthesis (theremin-like, flute-ish)

- **Plain Web Audio (recommended for v1, 0 KB):**
  - Glide one `OscillatorNode` per voice with `frequency.setTargetAtTime(f, t, 0.04–0.08)`, i.e. exponential
    portamento.
  - Use a `PeriodicWave` with a sine fundamental and weak 2nd and 3rd harmonics for a flute colour.
  - Add breath: white noise → `BiquadFilter` bandpass that tracks the note (Q about 5–10), mixed at about −24 dB.
  - Vibrato: an LFO at 5–6 Hz, ±10–20 cents, fading in after 300 ms of a held note.
  - Envelope: about 80 ms attack and 150 ms release, driven by the voice gate.
  - Reverb: a `ConvolverNode` with a synthetic impulse (1.5–2 s of exponentially decaying noise, generated in code,
    no file).
- **[Tone.js](https://tonejs.github.io/) 15.1.22** (MIT, about 5.4 MB unpacked, tree-shakeable): `MonoSynth` /
  `Synth` with a `portamento` property, plus `Reverb`, `Vibrato` and `Filter`. It's convenient, but the plain Web
  Audio version above is small enough that it isn't needed.
- **Physical model:** the Faust [`physmodels.lib`](https://faustlibraries.grame.fr/libs/physmodels/) `pm.flute_ui`
  (a reimplementation of the STK flute, which is under an STK-4.3 MIT-style licence).
  - Compile it **offline** with [`@grame/faustwasm`](https://github.com/grame-cncm/faustwasm) 0.19.0 (LGPL; about
    31.6 MB unpacked, but that's the compiler) into a precompiled WASM AudioWorklet.
  - At runtime the precompiled module needs no `libfaust-wasm`, and the README explicitly supports instances from
    precompiled modules.
  - Result: an expressive, breathy flute with real overblowing. Good for v2.
- **DDSP flute:** offline only (see A2).

### A4. Mobile realities

**Permissions and the user gesture**

- getUserMedia needs HTTPS.
- Create and `resume()` the `AudioContext` **synchronously inside the tap handler** ("Play the Auto Flute") *before*
  awaiting `getUserMedia`. On iOS a context created outside a gesture stays `suspended`, and contexts can
  re-suspend, so check `ctx.state` before playing (see
  [Sam Eddy, iOS Safari audio sessions](https://samueleddy.com/writing/ios-safari-audio-sessions/)).
- When the item closes, stop all tracks. This removes the recording indicator and is a privacy point in line with
  the owner's "no photos, no location" stance: **no audio leaves the device, and nothing is recorded**.

**iOS audio session**

- Mic plus Web Audio output puts Safari into `play-and-record`. Speaker output may then be routed to the earpiece
  or attenuated. This is a known, long-running set of WebKit bugs:
  [218012](https://bugs.webkit.org/show_bug.cgi?id=218012), [230902](https://bugs.webkit.org/show_bug.cgi?id=230902),
  [196539](https://bugs.webkit.org/show_bug.cgi?id=196539) (in iOS 15 the output dropped to about ½–⅔ volume).
- iOS 17+ supports the [Audio Session API](https://developer.mozilla.org/en-US/docs/Web/API/Audio_Session_API):
  set `navigator.audioSession.type = 'play-and-record'` explicitly, with feature detection.
- Per the post above, output from `AudioContext.destination` still reaches the speaker in that mode. Test on real
  iPhones; the simulator differs.
- Also check the ringer/silent switch behaviour on the test devices.

**Echo and feedback (the main design risk)**

- The harmony is a clean, steady tone close to the voice's range. If the mic hears it, MPM may lock onto it (it is
  "clearer" than a voice), the harmony then harmonises itself, and pitch runs away by thirds.
- Mitigations, in order:
  1. **Recommend headphones** on the item's start screen ("Mit Kopfhörern klingt's am besten").
  2. **Voice-gated output:** the flute only sounds while the voice gate is open, and the gate uses the mic RMS
     *minus* the expected bleed.
  3. **Self-rejection:** discard detections within ±50 cents of a frequency we are currently playing, unless the
     voice RMS rose.
  4. **Duck** the output by about 6 dB while the voice onset is detected.
  5. `echoCancellation: true` helps against bleed, but on iOS it forces the voice-processing path (earpiece and
     attenuation problems), and on Android AEC can also eat sustained sung tones. **Make it A/B testable**, and
     probably use `false` with headphones and `true` on speaker.
- **Fallback mode, "Echo-Flöte" (call and response):** you sing a phrase, then the flute answers with the harmonised
  phrase. This has no simultaneous mic and speaker, so no feedback at all. It also hosts the offline ML ideas
  (Coconet, DDSP flute) later.

**Latency**

- Native round trip: Pixel XL 13.5 ms, Pixel 2 18.1 ms
  ([Android source](https://source.android.com/docs/core/audio/latency/measurements)). The CDD target is ≤20 ms;
  older iPads measured 8–10 ms ([Superpowered](https://superpowered.com/latency)).
- Browsers add a lot on top. On a desktop Mac, Jeff Kaufman measured Chrome at 67 ms by default and **19 ms** with
  `latencyHint: 0` and processing off ([jefftk](https://www.jefftk.com/p/browser-audio-latency)).
- I found no trustworthy published phone-browser numbers. Commonly cited: **30–100 ms on mobile browsers
  (estimate)**. Chrome on Pixel once fell back to about 40 ms callbacks because AAudio wasn't in low-latency mode
  ([Chromium review](https://groups.google.com/a/chromium.org/g/feature-media-reviews/c/_a4oNpDGEYE)).
- Our budget: about 20–40 ms input and output plus 20–40 ms of analysis window plus about 30 ms of smoothing, so
  **about 80–150 ms from voice to harmony (estimate)**.
- That is too slow for a "tight" doubled voice, but fine for a gliding harmony that *follows*. Portamento masks the
  lag, and a third above is forgiving. Use `new AudioContext({ latencyHint: 'interactive' })` and measure
  `baseLatency` / `outputLatency`.
- Build a tiny round-trip test with the [Superpowered web test](https://github.com/superpoweredSDK/WebBrowserAudioLatencyMeasurement)
  or [weblatencytest](https://github.com/gilpanal/weblatencytest) method and run it on 2–3 of the owner's phones.

**Battery and CPU**

- MPM at about 100 Hz plus a few oscillators and one convolver is light. A CNN pitch model every 10 ms on WebGL is
  not.
- Stop everything (tracks, context `close()`) when the overlay closes, and suspend on `visibilitychange`.

### A5. Examples and demos to learn from

- **Blob Opera** (David Li / Google Arts & Culture, 2020): [experiment page](https://experiments.withgoogle.com/blob-opera),
  [play](https://artsandculture.google.com/experiment/blob-opera/AAHWrq360NcGbw). It shows a real-time ML
  harmoniser, scale-locked pitch and great UX for non-singers. It's closed source.
- **Chrome Music Lab**: [site](https://musiclab.chromeexperiments.com/). Song Maker has a mic input that turns sung
  pitch into notes. The code is at [googlecreativelab/chrome-music-lab](https://github.com/googlecreativelab/chrome-music-lab)
  (Apache-2.0, **archived**).
- **FreddieMeter**: [freddiemeter.withyoutube.com](https://freddiemeter.withyoutube.com/). It runs SPICE on the
  device, in mobile browsers too, with no upload ([Google SPICE post](https://research.google/blog/spice-self-supervised-pitch-estimation/)).
- **Tone Transfer** (Magenta + AIUX): [magenta.tensorflow.org/tone-transfer](https://magenta.tensorflow.org/tone-transfer).
  It turns a voice into a flute, offline in the browser, with SPICE plus DDSP.
- **Bach Doodle / Coconet**: [Magenta post](https://magenta.tensorflow.org/coconet), [paper](https://arxiv.org/abs/1907.06637).
  It shows how to make an ML harmoniser fast enough on consumer devices.
- **Piano Genie demo**: [magenta-js demo](https://magenta.github.io/magenta-js/music/demos/piano_genie.html).
- **Web theremins**: [mdn/violent-theremin](https://github.com/mdn/violent-theremin) (a minimal Web Audio theremin)
  and [lukephills/Theremin](https://github.com/lukephills/Theremin) (TypeScript).
- **Pitch detection**: the [pitchy playground](https://github.com/ianprime0509/pitchy) (linked from its README) and
  the [aubiojs demo](https://qiuxiang.github.io/aubiojs/).
- **Vocal arrangement (offline rules)**: [dffmonolith/harmonizer](https://github.com/dffmonolith/harmonizer).

### A6. Recommendation: architecture for v1

```
tap "Spielen" ──► AudioContext (resume in gesture) ──► getUserMedia({audio:{echoCancellation:?, noiseSuppression:false, autoGainControl:false}})
mic ─► MediaStreamSource ─► AudioWorklet "pitch-worklet" (pitchy MPM, 2048 window, 256-512 hop)
                                    │ postMessage {hz, clarity, rms, t}
main thread: smoothing + gate ─► note tracker ─► key estimator (or fixed key) ─► harmony rules (3rd above / 6th below, optional triad)
                                    │ target frequencies
synth (plain Web Audio): osc+PeriodicWave + breath noise + vibrato LFO ─► gain (voice-gated, ducked) ─► Convolver reverb ─► destination
UI: a theremin-like visual (sung line + harmony line on a staff/scale ladder), headphones hint
```

- **Lazy loading:** use the same pattern as `loadClassifier()`. A `loadAutoFlute()` dynamic `import()` pulls in the
  worklet module (`audioWorklet.addModule(new URL('./pitch-worklet.ts', import.meta.url))`), pitchy and the synth
  only when the item is used. **No ML download in v1** (well under 50 KB of JS).
- **Item model:** a new skill type (e.g. `play_flute`) in the catalog, with options like `key: "C"`,
  `harmony: "third" | "sixth" | "triad"` and `timbre: "flute"`. It needs no server state.
- **Prototype first (1–2 days):** use AnalyserNode + pitchy + one gliding oscillator a fixed third above in C major,
  on the owner's iPhone and an Android phone, through the speaker and then with headphones. This shows the feedback
  behaviour and the felt latency at once, and those are the two go/no-go questions. Then add the worklet, gating,
  self-rejection, key estimation and timbre.
- **Later:** a Faust `pm.flute_ui` precompiled AudioWorklet for the timbre; a call-and-response "Echo-Flöte" mode;
  and in that mode an optional offline Coconet (4-part) or DDSP flute re-render, ported to TF.js 4.
- **Risks:**
  - feedback on speaker (high)
  - iOS routing and volume quirks (high, and needs device testing)
  - latency feeling "laggy" (medium; masked by glide)
  - octave errors on breathy or low voices (medium; median filter)
  - licence traps: pitchfinder and aubio are GPL, PESTO is LGPL
  - Magenta's TF.js 2.7 pin

---

## Part B: Onion camera

### TL;DR

ImageNet-1k (our MobileNet) has **no onion, garlic, shallot, leek or scallion class**. The recommendation is to
**train a tiny custom classification head on the 1280-d MobileNet v2 embeddings we already compute (`embed()`)**:

- It's a linear softmax over {red, white, brown onion, other}, plus a nearest-prototype open-set check.
- It needs about 150–300 photos per onion colour and about 600–1000 negatives, with hard negatives (spring onion,
  shallot, garlic, apple, potato, ball).
- It ships as a 20–400 KB JSON file, lazy-loaded.
- It costs **zero extra model download and zero extra inference**: the same forward pass also feeds "Die kenn ich
  schon!".
- Zero-shot MobileCLIP-S0 (vision int8 11.8 MB, text prompts precomputed offline) is the fallback if the head isn't
  good enough.

### B1. ImageNet-1k check

I checked the class list shipped with tfjs-models
([`imagenet_classes.ts`](https://github.com/tensorflow/tfjs-models/blob/master/mobilenet/src/imagenet_classes.ts)).
It contains **none** of onion, garlic, shallot, leek, scallion or potato. WordNet / ImageNet-21k has onion synsets,
but they weren't in the 1k challenge set.

The vegetable and fruit classes are at indices 936–957: head cabbage, broccoli, cauliflower, zucchini, spaghetti
squash, acorn squash, butternut squash, cucumber, artichoke, bell pepper, cardoon, mushroom, Granny Smith,
strawberry, orange, lemon, fig, pineapple, banana, jackfruit, custard apple, pomegranate. Natural objects sit at
988–998 (acorn, hip, buckeye, …).

**Likely confusions (a hypothesis, to verify with step 1 below):**

| Onion | Likely top-5 neighbours in ImageNet-1k |
|---|---|
| red onion | `pomegranate` (957), `fig` (952), `bell pepper` (945), `head cabbage` (936, red-cabbage layers), `hip` (989) |
| brown / yellow onion | `buckeye, horse chestnut` (990), `acorn` (988), `orange` (950), `lemon` (951), `spaghetti squash` / `acorn squash` / `butternut squash` (940–942), `bagel` (931), `pretzel` (932) |
| white onion | `cauliflower` (938), `head cabbage` (936), `golf ball` (574), `ping-pong ball` (722), plus garlic-like objects with no class |
| halved / sliced | `artichoke` (944), `head cabbage`, `cauliflower` |
| spring onion (a negative) | `leek` doesn't exist, so most likely `ear, spike` (998), `cardoon`, `zucchini`, `cucumber` |

So no label set like the cucumber camera's `["cucumber", "zucchini"]` could work. The scores would be spread
thinly across fruit classes that also fire for real fruit.

### B2. Options compared

| # | Option | Extra download | Phone latency | Accuracy for onion vs spring onion / shallot / apple / potato | Effort | Verdict |
|---|---|---|---|---|---|---|
| 1 | **Custom head on our MobileNet v2 embeddings** (linear softmax or KNN, cf. [`@tensorflow-models/knn-classifier`](https://github.com/tensorflow/tfjs-models/tree/master/knn-classifier) 1.2.6, Apache-2.0, TF.js ^4.10) | **20 KB** (linear) to **about 400 KB** (with int8 prototypes) | **≈ 0**: the same `infer()` we already run for `embed()`, plus a 1280×4 dot product | Good for colour and shape classes given varied photos. Shallot vs small brown onion is genuinely hard (even for people) | Small: collect photos, train offline, ship JSON | **Recommended** |
| 2 | **Teachable Machine** ([`@teachablemachine/image`](https://www.npmjs.com/package/@teachablemachine/image) 0.8.5) | Its own truncated MobileNet plus head (a few MB) | similar | similar to 1 | Very easy to train | **Not for shipping.** It pins **`@tensorflow/tfjs` 1.3.1** (a second, very old runtime) and uses a different base model, so its head can't be reused on our embeddings. Fine as a quick feasibility check, since it trains in the browser from webcam photos |
| 3 | **Ready models with an onion class** | – | – | – | – | **None found.** Open Images V7: the image-level labels include `Onion` (/m/0dj75), `Red onion`, `Yellow onion`, `Pearl onion`, `Shallot`, `Scallion`, `Welsh onion`, `Garlic` and `Leek`, but the **600 boxable (detector) classes have no onion**, only `Potato`, `Vegetable`, `Apple` and `Ball` ([class lists](https://storage.googleapis.com/openimages/web/factsfigures_v7.html)). So the OIv7-trained YOLO and EfficientDet models can't detect onions. COCO has no onion either. Food-101 and the [AIY food classifier](https://aiyprojects.withgoogle.com/model/food-detector-classifier) are *dishes* (e.g. onion rings), not raw vegetables. [Fruits-360](https://github.com/Horea94/Fruit-Images-Dataset) has `Onion Red`, `Onion Red Peeled` and `Onion White`, but only on white backgrounds. It's a useful *extra* training source (licence: CC BY-SA 4.0 on Kaggle, MIT on GitHub/Mendeley). Open-vocabulary detectors (YOLO-World, OWL-ViT/OWLv2) could find "onion", but they are tens of MB plus ONNX Runtime Web (`onnxruntime-web` 1.30.0) and slow on phone CPUs |
| 4 | **Zero-shot CLIP-style** with [transformers.js](https://huggingface.co/docs/transformers.js) (`@huggingface/transformers` 4.3.0, Apache-2.0) | **MobileCLIP-S0 vision int8: 11.8 MB** ([files](https://huggingface.co/Xenova/mobileclip_s0/tree/main/onnx); fp16 22.9 MB). CLIP ViT-B/32 vision q4f16: 53 MB ([files](https://huggingface.co/Xenova/clip-vit-base-patch32/tree/main/onnx)). The text encoder (43–170 MB) is **not needed on the device**: precompute the prompt embeddings ("a photo of a red onion", "… spring onion", "… garlic", "… apple", "… potato", "… ball", …) offline and ship them as a few KB | MobileCLIP-S0 is "4.8× faster, 2.8× smaller than ViT-B/16 at comparable zero-shot accuracy" **(claimed)** ([Apple](https://machinelearning.apple.com/research/mobileclip)). CLIP B/32 runs at ">20 fps with WebGPU fp16" on a desktop GPU **(claimed)** ([Xenova](https://x.com/xenovacom/status/1782787876511441124)). On phones with WASM: **about 100–400 ms per frame (estimate)**, faster with WebGPU on recent Android Chrome | Should separate onion from spring onion well (the text knows the difference). Brown onion vs potato vs shallot is uncertain | Medium: a second runtime (ORT-Web WASM, several MB) next to TF.js | **Fallback** if option 1 underperforms. Also useful **offline as a labelling and bootstrap tool** |

A middle path if option 1 isn't accurate enough: **MediaPipe Model Maker** fine-tunes EfficientNet-Lite0 on our
photos into a TFLite classifier. It runs via [`@mediapipe/tasks-vision`](https://www.npmjs.com/package/@mediapipe/tasks-vision)
1.0.1 (Apache-2.0), with a model of about 4–5 MB **(estimate)** and its own WASM runtime. That's better accuracy
than a frozen-feature head, at the cost of a second runtime.

### B3. Option 1 in detail

**Classes:**

- `onion_red`, `onion_white`, `onion_brown` (yellow and brown together)
- `other`

For `other`, collect plenty of **hard negatives**: spring onion and leek, garlic, **shallot** (owner decision:
should a shallot count as an onion? botanically it's *Allium cepa* too), red apple, potato, ball, citrus, an onion
net bag without visible onions, and hands, tables and kitchen backgrounds.

**Photos:**

- Teachable-Machine-style demos work with 30–50 per class, but for a game that pays out items the false positive
  rate matters. Aim for **150–300 per onion class** and **600–1000 negatives** (about 100 per hard negative).
- Shoot them **with the phones and through the same aiming-square crop** we use at scan time (`cropSquare` →
  224×224).
- Vary the light, the background, whole vs halved, in hand vs on a counter vs in a net, and the distance.
- Add Open Images photos labelled `Red onion`, `Yellow onion` and `Onion` as a supplement (per-image licences,
  mostly CC BY 2.0; labels CC BY 4.0), and Fruits-360 sparingly.

**Training (offline, a dev script, not in the reader app):**

1. Compute embeddings with *exactly* our base model (MobileNet v2 1.0 via TF.js, `infer(x, true)` → 1280-d,
   L2-normalised like `embed()`). Use light augmentation: random crop and scale, flip, brightness and contrast.
2. Train a multinomial logistic regression (or one Dense layer) with class weights.
3. Hold out by *session or phone* rather than by random photo.
4. Pick the acceptance threshold so the negatives' false-positive rate is ≤1–2%.
5. Keep about 30–100 **prototypes** per onion class (k-means centroids) for an open-set check: reject if the
   cosine similarity to the nearest onion prototype is < τ. That stops "confident softmax on something never seen".

**Artefact:**

```json
// cdn: scutoons/models/onion-v1.json (~20 KB linear; + prototypes int8 ~100-400 KB)
{ "version": 1, "base": "mobilenet_v2_1.0_224", "dim": 1280,
  "classes": ["onion, red onion", "onion, white onion", "onion, brown onion", "other"],
  "W": "<base64 float32 or int8 + scale>", "b": [..], "prototypes": {...}, "tau": 0.62 }
```

Lazy-load it like the model. Version it and bump it when retrained. A change of base model invalidates it, so tie it
to `base`.

### B4. How it fits `scan_object`

`ScanObjectSkill` currently has `labels`, `grants`, `cooldown_s`, `per_day`, `threshold` and `map`. The proposal is
a minimal, backwards-compatible extension:

```ts
interface ScanObjectSkill {
  type: 'scan_object';
  /** custom head on the MobileNet embedding instead of ImageNet classes (e.g. "onion-v1"); none = ImageNet */
  model?: string;
  labels: string[];        // onion camera: ["onion"]  – matches "onion, red onion" etc. via matchLabels' word split
  grants: string;          // "onion"  (or map: red → red_onion … if the owner wants colours as separate items)
  cooldown_s?: number; per_day?: number; threshold?: number; map?: ...
}
```

- **Viewer** (`object-detector.ts`): `loadClassifier(model?)` reuses the cached MobileNet. For a `model`, it fetches
  the head JSON and returns a `Classifier` whose `classify()` runs `infer(source, true)`, normalises it, applies the
  head and returns `Prediction[]` (className from the head, softmax probability; `other` below the open-set τ).
  `embed()` stays the same and can reuse the same embedding (cache it per frame), so **the scan memory ("Die kenn
  ich schon!") works unchanged**.
  - `scoreScan` / `scanPasses` / `matchLabels` need no change, because the head's class names follow the ImageNet
    "comma list" convention.
  - The DEV hook `__scutoonsFakePredictions` keeps working.
- **Catalog and server:** allow `model` in `catalog.schema.json` / `items.ts` / `Catalog.php`, and validate that the
  name is a known head. The grant path is unchanged; the server already trusts the device's scan for these game
  items, with cooldown and `per_day`.
- **Item:** `onion_camera` (Top Toy's camera), a recipe or story grant as the owner decides, with a cooldown and
  `per_day` like the cucumber camera.

### B5. What to do first

1. **Verify the hypothesis in 30 minutes:** point the existing cucumber camera's classify at 20–30 onion photos and
   log the top-5 (DEV). This confirms B1 and gives a baseline.
2. **Feasibility check:** Teachable Machine in a desktop browser with about 50 photos per class (onion colours,
   spring onion, garlic, apple, potato). If TM can't separate them, a frozen-MobileNet head won't either. Go
   straight to MobileCLIP or Model Maker.
3. **Build the photo set and the dev training script.** Ship `onion-v1.json` behind a DEV flag, test on the owner's
   phones with the real aiming square, then tune the threshold and τ.

**Risks:**

- shallot vs onion ambiguity (a product decision)
- onions in nets or bags
- lighting (yellow kitchen light makes white onions look brown)
- the frozen-feature ceiling: if accuracy is poor, go to MediaPipe Model Maker or MobileCLIP-S0 at the cost of a
  second runtime

---

## Sources (main)

- pitchy: https://github.com/ianprime0509/pitchy ; npm `pitchy` 4.1.0
- pitchfinder (GPL-3): https://www.npmjs.com/package/pitchfinder ; aubiojs: https://github.com/qiuxiang/aubiojs , aubio (GPL-3): https://github.com/aubio/aubio
- CREPE: https://github.com/marl/crepe ; ml5 CREPE weights: https://github.com/ml5js/ml5-data-and-models/tree/main/models/pitch-detection/crepe ; ml5 issue on the tiny model: https://github.com/ml5js/ml5-library/issues/975
- SPICE: https://arxiv.org/abs/1910.11664 , https://blog.tensorflow.org/2020/06/estimating-pitch-with-spice-and-tensorflow-hub.html
- Basic Pitch: https://github.com/spotify/basic-pitch-ts , size notes: https://dev.to/partfit3d/running-spotifys-basic-pitch-in-the-browser-for-audio-to-midi-44bc
- PESTO: https://arxiv.org/abs/2508.01488 , https://github.com/SonyCSLParis/pesto
- Magenta.js: https://github.com/magenta/magenta-js , https://magenta.github.io/magenta-js/music/ ; Coconet: https://magenta.tensorflow.org/coconet , https://arxiv.org/abs/1907.06637 ; Piano Genie: https://arxiv.org/abs/1810.05246 ; Tone Transfer: https://magenta.tensorflow.org/tone-transfer ; DDSP-VST: https://magenta.withgoogle.com/ddsp-vst-blog
- AI Harmonizer (NIME 2025): https://arxiv.org/abs/2506.18143
- Tone.js: https://tonejs.github.io/ ; Faust: https://github.com/grame-cncm/faustwasm , https://faustlibraries.grame.fr/libs/physmodels/
- Latency: https://source.android.com/docs/core/audio/latency/measurements , https://superpowered.com/latency , https://www.jefftk.com/p/browser-audio-latency , https://github.com/gilpanal/weblatencytest
- iOS audio: https://developer.mozilla.org/en-US/docs/Web/API/Audio_Session_API , https://samueleddy.com/writing/ios-safari-audio-sessions/ , WebKit bugs 218012 / 230902 / 196539
- Demos: https://experiments.withgoogle.com/blob-opera , https://musiclab.chromeexperiments.com/ , https://freddiemeter.withyoutube.com/ , https://github.com/mdn/violent-theremin
- ImageNet classes: https://github.com/tensorflow/tfjs-models/blob/master/mobilenet/src/imagenet_classes.ts
- Open Images V7 class lists: https://storage.googleapis.com/openimages/v7/oidv7-class-descriptions-boxable.csv , https://storage.googleapis.com/openimages/v7/oidv7-class-descriptions.csv
- Fruits-360: https://github.com/Horea94/Fruit-Images-Dataset
- KNN classifier: https://github.com/tensorflow/tfjs-models/tree/master/knn-classifier ; Teachable Machine lib: https://www.npmjs.com/package/@teachablemachine/image
- MobileCLIP: https://machinelearning.apple.com/research/mobileclip , https://huggingface.co/Xenova/mobileclip_s0 ; CLIP ONNX: https://huggingface.co/Xenova/clip-vit-base-patch32
- MediaPipe tasks-vision: https://www.npmjs.com/package/@mediapipe/tasks-vision
