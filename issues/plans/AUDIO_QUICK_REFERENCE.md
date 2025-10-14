# Audio System Quick Reference

## ✅ Audio Lifecycle

When leaving a chapter view, the audio system gracefully shuts down:

### Music Tracks
1. **Fade out** over the default transition duration (3000ms)
2. **Stop playback** after fade completes

### Sample Sounds
1. **Fade out** any playing samples (500ms or less)
2. **Stop all samples** after fade completes

### Cleanup
- **Parallel fade-out** - Music and samples fade simultaneously
- **Unload all audio** - Frees memory for the next chapter
- **Release audio context** - Ready for reuse

This ensures smooth transitions and proper resource management.

## 🎵 Adding Audio to a Chapter

### Minimal Setup (Music Only)
```json
{
  "title": "Chapter 1",
  "audio": {
    "music": {
      "defaultTrack": "calmness",
      "tracks": [
        {"id": "calmness", "path": "assets/audio/music/calm.mp3"}
      ]
    }
  },
  "slices": [
    {"image": "slice1.jpg"}
  ]
}
```

**Note**: `defaultTrack` is optional. If not specified, the first track in the array will be used as default.

### Full Adaptive Setup
```json
{
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
      {"id": "door", "path": "assets/audio/samples/door.mp3"}
    ]
  },
  "slices": [
    {
      "image": "slice1.jpg",
      "audio": {
        "music": {
          "tracks": [
            {"id": "calmness", "volume": 1.0},
            {"id": "curiousness", "volume": 0.0},
            {"id": "tension", "volume": 0.0},
            {"id": "excitement", "volume": 0.0}
          ]
        },
        "samples": [
          {
            "id": "footstep",
            "trigger": {"type": "scrollInView", "mode": "mono"},
            "volume": 0.7,
            "delay": 0
          }
        ]
      }
    },
    {
      "image": "slice2.jpg",
      "audio": {
        "music": {
          "transitionDuration": 2000,
          "tracks": [
            {"id": "calmness", "volume": 0.3},
            {"id": "tension", "volume": 0.7}
          ]
        },
        "samples": [
          {
            "id": "door",
            "trigger": {"type": "click", "mode": "poly"},
            "volume": 1.0
          }
        ]
      }
    }
  ]
}
```

## 🔑 Key Concepts

### Music Tracks
- **Chapter level**: Define which audio files exist
- **Slice level**: Control volume levels for each track
- **Crossfade**: Automatic smooth transitions between slices

### Sample Library
- **Chapter level**: Define sample files for preloading
- **Slice level**: Configure when and how samples trigger

### Trigger Types
- **scrollInView**: Auto-play when slice is 50% visible
- **click**: Play when user clicks on slice
- **hover**: Play when user hovers over slice

### Playback Modes
- **mono**: Stop previous playback before starting new
- **poly**: Allow multiple simultaneous playbacks

## 📐 Common Patterns

### Calm to Tense Transition
```json
// Slice 1 - Calm
"audio": {
  "music": {
    "tracks": [
      {"id": "calmness", "volume": 1.0},
      {"id": "tension", "volume": 0.0}
    ]
  }
}

// Slice 2 - Building tension
"audio": {
  "music": {
    "tracks": [
      {"id": "calmness", "volume": 0.5},
      {"id": "tension", "volume": 0.5}
    ]
  }
}

// Slice 3 - Full tension
"audio": {
  "music": {
    "tracks": [
      {"id": "calmness", "volume": 0.0},
      {"id": "tension", "volume": 1.0}
    ]
  }
}
```

### Background Ambient + Action Moments
```json
"audio": {
  "music": {
    "tracks": [
      {"id": "calmness", "volume": 0.3},
      {"id": "excitement", "volume": 0.0}
    ]
  },
  "samples": [
    {
      "id": "explosion",
      "trigger": {"type": "scrollInView", "mode": "mono"},
      "volume": 1.0,
      "delay": 500
    }
  ]
}
```

### Interactive Hotspot
```json
"samples": [
  {
    "id": "secret_sound",
    "trigger": {
      "type": "click",
      "mode": "poly",
      "rect": {
        "x": 20,
        "y": 30,
        "width": 15,
        "height": 15
      }
    },
    "volume": 0.8
  }
]
```

## 🎛️ Parameter Ranges

| Parameter | Range | Default | Notes |
|-----------|-------|---------|-------|
| volume | 0.0 - 1.0 | 1.0 | 0 = silent, 1 = full |
| transitionDuration | 0 - 10000 | 3000 | Milliseconds |
| delay | 0 - 10000 | 0 | Milliseconds |
| pan | -1.0 - 1.0 | 0 | -1 = left, 1 = right |
| rect.x, y, width, height | 0 - 100 | - | Percentage |

## 🎹 Keyboard Shortcuts

- **M**: Toggle mute/unmute

## 🐛 Troubleshooting

| Issue | Solution |
|-------|----------|
| Audio doesn't start | Click or scroll on page first (autoplay policy) |
| Crossfade is choppy | Increase transitionDuration to 3000+ |
| Sample plays too many times | Use "mode": "mono" or "gate": true |
| Volume too loud/soft | Adjust volume parameter (0.5-0.8 is good) |
| Can't hear certain emotions | Check all track volumes add up reasonably |

## 📁 File Requirements

### Audio Files
- **Format**: MP3 (recommended), OGG, WAV
- **Bitrate**: 128-192 kbps (sufficient quality)
- **Music tracks**: Should be same length and tempo
- **Samples**: Keep under 2 seconds for best performance

### File Structure
```
catpope/
├── assets/
│   └── audio/
│       ├── music/
│       │   ├── s1c1-instrumental-brass-morton.mp3
│       │   └── s1c2-instrumental.mp3
│       └── samples/
│           ├── s1c1-bells.mp3
│           ├── s1c1-choir-loud-loud-loud.mp3
│           └── s1c1-choir-once-it-means-nothing.mp3
└── api/static/catpope/de/
    └── chapter1.json
```

## 🧪 Testing Your Audio

1. Open browser console (F12)
2. Load chapter with audio
3. Look for: "AudioManager initialized successfully"
4. Click/scroll to start audio
5. Check console for any errors
6. Adjust volumes based on listening

## 💡 Tips

- **Start simple**: Begin with default track only
- **Test crossfades**: Use obvious different music to test transitions
- **Volume levels**: Keep music at 0.6-0.8, samples at 0.7-1.0
- **Transition duration**: 2000-4000ms feels natural
- **Sample timing**: Add 200-500ms delay for dramatic effect
- **Mobile**: Reduce sample count (mobile devices are less powerful)

## 📖 Full Documentation

See `catpope/app/audio/README.md` for complete API reference.
