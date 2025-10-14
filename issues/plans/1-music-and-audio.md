# Music and Audio System Implementation Plan

## Overview

Implementation of a dual audio system for SCUTOONS chapters:
1. **Adaptive Music Player** - 4-parallel-track emotion-based music system
2. **Sample Trigger Manager** - Event-based audio sample playback

**Technology**: Howler.js (CDN) for audio playback and management

---

## Data Structure

### Chapter Level - Asset Definitions

```json
{
  "title": "Chapter 1",
  "audio": {
    "music": {
      "transitionDuration": 3000,
      "tracks": [
        {"id": "default", "path": "path/to/default.mp3"},
        {"id": "calmness", "path": "path/to/calm.mp3"},
        {"id": "curiousness", "path": "path/to/curious.mp3"},
        {"id": "tension", "path": "path/to/tension.mp3"},
        {"id": "excitement", "path": "path/to/excitement.mp3"}
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

### Slice Level - Behavior Definitions

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

---

## Architecture

### Module Structure

```
catpope/app/audio/
├── AudioManager.js              Master coordinator
├── AdaptiveMusicPlayer.js       4-track emotion system
├── SampleTriggerManager.js      Event-based samples
└── index.js                     Exports
```

### AudioManager.js
**Purpose**: Master coordinator for all audio

**Responsibilities**:
- Initialize Howler global settings
- Coordinate between music player and sample manager
- Handle master volume/mute controls
- Manage autoplay policy

**API**:
```javascript
class AudioManager {
  constructor()
  async init(audioConfig)
  setMasterVolume(volume)
  mute() / unmute()
  getMusicPlayer()
  getSampleManager()
  cleanup()
}
```

### AdaptiveMusicPlayer.js
**Purpose**: 4-parallel-track emotion-based music

**Responsibilities**:
- Load and manage music tracks (or fallback to default)
- Handle smooth crossfading based on slice data
- Observe scroll position for slice transitions
- Sync all tracks on start

**API**:
```javascript
class AdaptiveMusicPlayer {
  constructor(audioManager)
  async loadTracks(musicConfig)
  start()
  updateVolumes(tracks, duration)
  pause() / resume()
  stop()
  getCurrentVolumes()
}
```

### SampleTriggerManager.js
**Purpose**: Event-based sample playback

**Responsibilities**:
- Load and cache sample library
- Detect trigger events (scrollInView, click, hover)
- Handle trigger modes (mono/poly, gating, exclusivity)
- Apply audio parameters (delay, volume, pan, pitch, envelope)

**API**:
```javascript
class SampleTriggerManager {
  constructor(audioManager)
  async loadSampleLibrary(sampleLibrary)
  registerSliceTriggers(sliceElement, sampleConfigs)
  trigger(sampleId, config)
  cleanup()
}
```

---

## Implementation Phases

### Phase 1: Foundation (~30 min) ✅
- [x] Add Howler.js CDN to index.html
- [x] Create audio/ directory structure
- [x] Build AudioManager.js with Howler integration
- [x] Create audio/index.js exports

### Phase 2: Adaptive Music Player (~60 min) ✅
- [x] Build AdaptiveMusicPlayer.js
  - [x] Track loading with Howler
  - [x] Crossfade logic using `.fade()`
  - [x] Handle default track fallback
- [x] Add scroll observation for slice transitions
- [x] Test with sample audio files

### Phase 3: Sample Trigger Manager (~90 min) ✅
- [x] Build SampleTriggerManager.js
  - [x] Sample library loading and caching
  - [x] Trigger detection (IntersectionObserver, click, hover)
  - [x] Mono/poly mode handling
  - [x] Gating and exclusivity logic
  - [x] Envelope implementation
- [x] Modify ChapterSlice.js to add trigger data attributes
- [x] Test with sample sound effects

### Phase 4: Integration (~45 min) ✅
- [x] Modify ChapterView.js to initialize audio system
- [x] Coordinate audio preloading with image loading
- [x] Add error handling and fallbacks
- [x] Handle backwards compatibility (chapters without audio)

### Phase 5: UI Controls (~30 min) ✅
- [x] Create AudioControls.js component
  - [x] Master volume slider
  - [x] Mute toggle
  - [x] Loading indicator
- [x] Add controls to chapter view
- [x] Handle autoplay policy (show play button if blocked)
- [x] Add keyboard shortcuts

## ✅ Implementation Status: COMPLETE

All phases have been successfully implemented. The audio system is ready for testing with actual audio files.

---

## Key Features

### Performance Optimizations
1. **Single Audio Context** - Howler manages this automatically
2. **Audio Caching** - All samples cached after first load
3. **IntersectionObserver** - Efficient scroll trigger detection
4. **Debounced Events** - Prevent trigger spam
5. **Lazy Trigger Registration** - Only for visible/near slices

### Browser Compatibility
- **Autoplay Policy** - Detect and handle with user gesture requirement
- **Mobile Optimization** - Howler automatically uses HTML5 Audio on iOS
- **Fallback** - Graceful degradation if audio fails

### User Experience
- **Backwards Compatible** - Chapters without audio work normally
- **Progressive Loading** - Audio preloads don't block image display
- **Error Recovery** - Fallback to default track or silent mode
- **User Controls** - Volume, mute, and playback controls

---

## Testing Strategy

### Unit Tests
- AudioManager initialization and cleanup
- Track loading and volume control
- Sample caching and playback
- Trigger detection logic

### Integration Tests
- Music crossfading during scroll
- Sample triggering on slice visibility
- Multiple samples playing simultaneously
- Autoplay policy handling

### Manual Testing
- Test with various audio formats (mp3, ogg, wav)
- Test on mobile devices (iOS, Android)
- Test with slow network (preloading)
- Test with chapters: no audio, music only, samples only, full system

---

## File Changes

### New Files
- `catpope/app/audio/AudioManager.js`
- `catpope/app/audio/AdaptiveMusicPlayer.js`
- `catpope/app/audio/SampleTriggerManager.js`
- `catpope/app/audio/index.js`
- `catpope/app/components/AudioControls.js`

### Modified Files
- `catpope/index.html` - Add Howler.js CDN
- `catpope/app/views/ChapterView.js` - Initialize audio system
- `catpope/app/components/ChapterSlice.js` - Add audio trigger attributes
- `catpope/app/utils/imageLoader.js` - Coordinate with audio preloading (optional)

---

## Future Enhancements

### Phase 6+ (Post-MVP)
- [ ] Audio waveform visualization
- [ ] Spatial audio (3D positioning)
- [ ] Real-time audio effects (reverb, filters)
- [ ] Music tempo sync with scroll speed
- [ ] Audio analytics (engagement tracking)
- [ ] Audio compression/optimization pipeline
- [ ] Offline audio caching (service worker)

---

## Notes

- Howler.js will be loaded via CDN for now
- Later integration with Electron app will bundle Howler
- All audio paths relative to chapter JSON location
- Sample library preloads all samples upfront
- Slices define when and how samples trigger
- Music system runs continuously with volume crossfades
- Backwards compatible: chapters without audio work normally
