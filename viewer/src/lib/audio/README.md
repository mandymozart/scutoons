# SCUTOONS Audio System

A modular audio system for SCUTOONS chapters featuring adaptive music and event-based sample triggers.

## Features

### 🎵 Adaptive Music Player
- 4 parallel emotion-based music tracks (calmness, curiousness, tension, excitement)
- Smooth crossfading based on scroll position
- Per-slice volume control
- Automatic loop synchronization

### 🔊 Sample Trigger Manager
- Event-based audio sample playback
- Multiple trigger types: intersect, click, hover
- Mono/poly modes for sample playback
- Gating and exclusivity options
- Audio envelopes (attack, sustain, release)

### 🎛️ Audio Controls
- Master volume control
- Mute/unmute toggle
- Keyboard shortcuts (M for mute)
- Floating UI controls

## Architecture

### Components

```
Svelte Integration (New)
├── contexts/audio.svelte.ts (Svelte wrapper with lifecycle management)
│   └── AudioManager (Master Coordinator)
│       ├── AdaptiveMusicPlayer (4-track parallel music)
│       │   └── Howl instances (background music tracks)
│       └── SampleTriggerManager (Sound effects)
│           └── Howl instances (one-shot samples)
└── actions/audio.ts (Svelte actions for DOM triggers)
```

### Why This Design?
- **Framework Independence**: AudioManager is reusable across frameworks
- **Svelte Integration**: Context + Actions pattern for reactive integration
- **Single AudioContext**: Browsers limit concurrent audio contexts
- **Centralized Control**: Master volume/mute affects all audio
- **Lazy Loading**: Only load audio files when needed
- **Modular**: Music and samples are independent systems
- **Graceful Cleanup**: Audio fades out before stopping when leaving chapter
- **Per-Chapter Configuration**: Each chapter can have distinct audio configs

### Data Flow

**Chapter Load:**
1. **Chapter loads** → Audio config extracted
2. **AudioManager** → Initializes music player and sample manager
3. **Music Player** → Loads and starts all tracks at volume 0
4. **Slice Observer** → Detects slice visibility changes
5. **Music transitions** → Crossfades to new emotion mix
6. **Sample triggers** → Fires based on configured events

**Chapter Exit:**
1. **User navigates away** → Cleanup triggered
2. **Music fades out** → 3000ms (default transition duration)
3. **Samples fade out** → 500ms (shorter for sound effects)
4. **Parallel cleanup** → Both fade simultaneously
5. **Resources released** → Audio context ready for next chapter

## Usage

### Svelte Integration (Updated)

#### 1. Layout Setup (`[toonId]/+layout.svelte`)

```svelte
<script lang="ts">
  import { setAudioContext } from '$lib/contexts/audio.svelte.ts';
  import { onDestroy } from 'svelte';

  const audio = setAudioContext();

  onDestroy(async () => {
    await audio.unload();
  });
</script>

{@render children()}
```

#### 2. Chapter Page (`chapter/[chapterNumber]/+page.svelte`)

```svelte
<script lang="ts">
  import { getAudioContext } from '$lib/contexts/audio.svelte.ts';
  import { untrack } from 'svelte';

  let { data } = $props();
  const audio = getAudioContext();

  $effect(() => {
    const chapter = data.chapter;

    if (chapter.audio) {
      audio.loadConfiguration(chapter.audio).then(() => {
        audio.startMusic();
      });
    }

    return () => {
      untrack(() => {
        audio.unload();
      });
    };
  });
</script>
```

#### 3. Slice Component with Audio Triggers

```svelte
<script lang="ts">
  import { audioTrigger } from '$lib/actions/audio.ts';
  import { asset } from '$lib/utils/cdn.ts';

  let { slice } = $props();
</script>

{#if slice.audio?.samples}
  <div class="slice" use:audioTrigger={slice.audio.samples[0]}>
    <img src={asset(slice.image)} alt="Slice" />
  </div>
{:else}
  <div class="slice">
    <img src={asset(slice.image)} alt="Slice" />
  </div>
{/if}
```

#### 4. Manual Sample Triggering (Visual Novel Events)

```svelte
<script lang="ts">
  import { getAudioContext } from '$lib/contexts/audio.svelte.ts';

  const audio = getAudioContext();

  function onVNEvent(event) {
    if (event.audio) {
      audio.triggerSample(event.audio.sample);
    }
  }
</script>

<button onclick={() => audio.triggerSample({ id: 'click_sound' })}>
  Click Me
</button>
```

#### 5. Music Volume Control (Scroll-based Emotion Changes)

```svelte
<script lang="ts">
  import { getAudioContext } from '$lib/contexts/audio.svelte.ts';

  const audio = getAudioContext();

  function updateEmotion(emotion: 'calm' | 'tense') {
    if (emotion === 'calm') {
      audio.updateMusicVolumes([
        { id: 'calmness', volume: 1.0 },
        { id: 'tension', volume: 0.0 }
      ], 2000);
    } else {
      audio.updateMusicVolumes([
        { id: 'calmness', volume: 0.0 },
        { id: 'tension', volume: 1.0 }
      ], 2000);
    }
  }
</script>
```

### Chapter JSON Structure

```json
{
  "title": "Chapter 1",
  "audio": {
    "music": {
      "transitionDuration": 3000,
      "defaultTrack": "calmness",
      "tracks": [
        {"id": "calmness", "path": "assets/audio/music/calm.mp3"},
        {"id": "curiousness", "path": "assets/audio/music/curious.mp3"},
        {"id": "tension", "path": "assets/audio/music/tension.mp3"},
        {"id": "excitement", "path": "assets/audio/music/excitement.mp3"}
      ]
    },
    "sampleLibrary": [
      {"id": "footstep", "path": "assets/audio/samples/footstep.mp3"},
      {"id": "door_slam", "path": "assets/audio/samples/door.mp3"}
    ]
  },
  "slices": [...]
}
```

**Note**: The `defaultTrack` property specifies which track plays at full volume when no slice audio configuration is present. If omitted, the first track in the array is used as default.

### Slice Audio Configuration

```json
{
  "image": "path/to/image.jpg",
  "audio": {
    "music": {
      "transitionDuration": 2000,
      "tracks": [
        {"id": "calmness", "volume": 0.8},
        {"id": "curiousness", "volume": 0.2},
        {"id": "tension", "volume": 0.0},
        {"id": "excitement", "volume": 0.0}
      ]
    },
    "samples": [
      {
        "id": "footstep",
        "trigger": {
          "type": "intersect",
          "gate": false,
          "exclusive": false,
          "mode": "mono",
          "rect": {
            "x": 0,
            "y": 0,
            "width": 100,
            "height": 100
          }
        },
        "delay": 500,
        "volume": 0.7,
        "pan": 0,
        "pitch": 0,
        "loop": false,
        "envelope": {
          "attack": 100,
          "sustain": 500,
          "release": 200
        }
      }
    ]
  }
}
```

## API Reference

### Audio Context (Svelte)

```typescript
import { getAudioContext } from '$lib/contexts/audio.svelte.ts';

const audio = getAudioContext();

// Lifecycle
await audio.loadConfiguration(audioConfig);
await audio.unload();

// Music control
audio.startMusic();
audio.pauseMusic();
audio.resumeMusic();
audio.updateMusicVolumes([
  { id: "calmness", volume: 0.8 },
  { id: "tension", volume: 0.2 }
], 3000);

// Sample control
audio.triggerSample({ id: 'footstep' });

// Volume control
audio.setVolume(0.8);      // 0.0 to 1.0
audio.toggleMute();

// Reactive state
audio.state.isInitialized  // boolean
audio.state.isMuted        // boolean
audio.state.masterVolume   // number
audio.state.isPlaying      // boolean
audio.state.currentConfig  // AudioConfiguration | null

// Direct AudioManager access (if needed)
audio.manager.getMusicPlayer();
audio.manager.getSampleManager();
```

### Audio Actions (Svelte)

```typescript
import { audioTrigger } from '$lib/actions/audio.ts';

// Usage in template
<div use:audioTrigger={sampleConfig}>
  Content
</div>

// Sample config structure
const sampleConfig = {
  id: 'footstep',
  trigger: {
    type: 'intersect',  // 'click' | 'hover' | 'intersect'
    mode: 'mono',          // 'mono' | 'poly'
    gate: false,
    exclusive: false,
    rect: {                // Optional: hit area
      x: 0,
      y: 0,
      width: 100,
      height: 100
    }
  },
  volume: 0.7,
  delay: 500,
  loop: false
};
```

### AudioManager (Direct Access)

```typescript
// Low-level API (typically not needed in Svelte components)
const audioManager = new AudioManager();
await audioManager.init(audioConfig);

// Volume control
audioManager.setMasterVolume(0.8);
audioManager.getMasterVolume();

// Mute control
audioManager.toggleMute();

// Access subsystems
const musicPlayer = audioManager.getMusicPlayer();
const sampleManager = audioManager.getSampleManager();

// Cleanup
await audioManager.cleanup();
```

## Configuration Options

### Music Track

| Property | Type | Description | Default |
|----------|------|-------------|---------|
| `id` | string | Unique track identifier | Required |
| `path` | string | Path to audio file | Required |

### Music Configuration

| Property | Type | Description | Default |
|----------|------|-------------|---------|
| `transitionDuration` | number | Crossfade duration (ms) | 3000 |
| `defaultTrack` | string | ID of track to play at full volume by default | First track |
| `tracks` | array | Available music tracks | Required |

### Sample Configuration

| Property | Type | Description | Default |
|----------|------|-------------|---------|
| `id` | string | Sample identifier | Required |
| `trigger.type` | string | 'intersect', 'click', 'hover' | 'intersect' |
| `trigger.gate` | boolean | Only play if not already playing | false |
| `trigger.exclusive` | boolean | Mute other audio | false |
| `trigger.mode` | string | 'mono' or 'poly' | 'mono' |
| `trigger.rect` | object | Hit area (x, y, width, height %) | Full slice |
| `delay` | number | Delay before playing (ms) | 0 |
| `volume` | number | Sample volume (0.0-1.0) | 1.0 |
| `pan` | number | Stereo pan (-1 to 1) | 0 |
| `pitch` | number | Pitch shift (semitones) | 0 |
| `loop` | boolean | Loop playback | false |
| `envelope` | object | Attack, sustain, release (ms) | - |

## Browser Compatibility

- **Modern browsers**: Full support (Web Audio API)
- **Mobile Safari**: Requires user gesture to start audio
- **Autoplay policy**: Handled automatically with user interaction detection

## Performance

### Optimizations
- Music is **streamed** (2026-09-30, agents/PERFORMANCE.md P1): one `<audio>` element per track routed through its own
  Web Audio output (`MediaElementAudioSourceNode` → gain per track → music master gain; one AudioContext per page,
  never closed, because Howler replaces its own context). Fades are gain ramps (they work on iOS, where
  `audio.volume` is read-only – the reason not to use Howler `html5: true`). The master volume / mute apply to it too.
- The chapter's audio starts loading only after the media of the first 2 slices has loaded (max. 4 s wait,
  `afterFirstSlices` in `lib/actions/audio`). Volumes the slices in view ask for meanwhile are remembered and
  applied (faded in) when the music starts.
- Samples: Web Audio buffers via Howler (loaded once, short latency), in parallel with the music
- IntersectionObserver for efficient scroll detection
- Lazy trigger registration

### Resource Usage
- **Memory**: samples decoded (~1-3 MB), the music is not decoded up front
- **CPU**: Minimal (~1-2% during crossfades)
- **Network**: music 128 kbps MP3 (`*.128.mp3`), buffered progressively; samples downloaded in full

## Troubleshooting

### Audio doesn't play
- Check browser autoplay policy (requires user gesture)
- Verify audio files are accessible
- Check browser console for errors

### Music doesn't crossfade smoothly
- Ensure all tracks have same tempo and length
- Check `transitionDuration` is sufficient (recommend 2000-5000ms)
- Verify tracks are properly looped

### Samples trigger multiple times
- Use `mode: "mono"` to prevent overlapping playback
- Enable `gate: true` to block triggers while playing

### Performance issues
- Reduce number of simultaneous samples
- Use shorter audio files
- Compress audio files (mp3 @ 128kbps is sufficient)

## Development

### Adding New Trigger Types

1. Add trigger type to `SampleTriggerManager.registerTrigger()`
2. Implement trigger detection method
3. Return cleanup function
4. Update documentation

### Extending Music System

- Additional emotion states: Add new tracks to config
- Custom crossfade curves: Modify `StreamedTrack.fade()` in `AdaptiveMusicPlayer.ts`
- Dynamic tempo: Integrate with scroll speed

## Testing

### Manual Testing Checklist
- [ ] Music starts on first user interaction
- [ ] Volume crossfades smoothly between slices
- [ ] Samples trigger at correct events
- [ ] Audio controls work (volume, mute)
- [ ] Keyboard shortcuts function (M for mute)
- [ ] Mobile devices handle autoplay policy
- [ ] Cleanup runs when navigating away
- [ ] Backwards compatible (chapters without audio)

### Test Chapter Example

See `issues/plans/1-music-and-audio.md` for complete data structure examples.

## Dependencies

- **Howler.js** v2.2.4 (loaded via CDN)
- No other external dependencies

## License

Part of the SCUTOONS project.
