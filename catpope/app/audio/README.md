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
- Multiple trigger types: scrollInView, click, hover
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
audio/
├── AudioManager.js              # Master coordinator
├── AdaptiveMusicPlayer.js       # 4-track emotion system
├── SampleTriggerManager.js      # Event-based samples
└── index.js                     # Module exports
```

### Data Flow

1. **Chapter loads** → Audio config extracted
2. **AudioManager** → Initializes music player and sample manager
3. **Music Player** → Loads and starts all tracks at volume 0
4. **Slice Observer** → Detects slice visibility changes
5. **Music transitions** → Crossfades to new emotion mix
6. **Sample triggers** → Fires based on configured events

## Usage

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
          "type": "scrollInView",
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

### AudioManager

```javascript
const audioManager = new AudioManager();
await audioManager.init(audioConfig);

// Volume control
audioManager.setMasterVolume(0.8);      // 0.0 to 1.0
audioManager.getMasterVolume();

// Mute control
audioManager.mute();
audioManager.unmute();
audioManager.toggleMute();

// Access subsystems
const musicPlayer = audioManager.getMusicPlayer();
const sampleManager = audioManager.getSampleManager();

// Cleanup
audioManager.cleanup();
```

### AdaptiveMusicPlayer

```javascript
const musicPlayer = audioManager.getMusicPlayer();

// Start playback
musicPlayer.start();

// Update volumes (crossfade)
musicPlayer.updateVolumes([
  {id: "calmness", volume: 0.8},
  {id: "tension", volume: 0.2}
], 3000); // 3 second crossfade

// Playback control
musicPlayer.pause();
musicPlayer.resume();
musicPlayer.stop();

// State
const volumes = musicPlayer.getCurrentVolumes();
const isPlaying = musicPlayer.isCurrentlyPlaying();
```

### SampleTriggerManager

```javascript
const sampleManager = audioManager.getSampleManager();

// Register triggers for a slice
sampleManager.registerSliceTriggers(sliceElement, sampleConfigs);

// Manual trigger
sampleManager.trigger(sampleConfig);

// Cleanup
sampleManager.cleanup();
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
| `trigger.type` | string | 'scrollInView', 'click', 'hover' | 'scrollInView' |
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
- Single Web Audio context (no resource limits)
- Sample caching (loaded once, reused multiple times)
- IntersectionObserver for efficient scroll detection
- Debounced event handlers
- Lazy trigger registration

### Resource Usage
- **Memory**: ~5-10 MB per chapter (depends on audio file sizes)
- **CPU**: Minimal (~1-2% during crossfades)
- **Network**: All audio preloaded before chapter starts

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
- Custom crossfade curves: Modify `Howl.fade()` implementation
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
