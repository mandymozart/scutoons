# Adaptive Music System

## Overview

A 4-state emotion-based adaptive music system that smoothly transitions between musical moods based on scene parameters.

## Emotion States

The system uses four parallel music states:

- **Calmness** - Safe, peaceful atmosphere
- **Curiousness** - Exploration and discovery
- **Tension** - Danger and suspense
- **Excitement** - Triumph and intensity

## Architecture

### Audio Players

Four synchronized audio players run continuously in parallel, one per emotion state. Each plays a looping musical stem in the same tempo and key signature.

```json
{
  //...
  "audio": {
    "music": {
      "transitionDuration": 3000, // ms - crossfade duration (global, defaults to 3000ms)
      "tracks": [
        {"id": "default", "path": "path/to/default.mp3"}, // fallback track (required, used for single track scutoons)
        {"id": "calmness", "path": "path/to/calm.mp3"},
        {"id": "curiousness", "path": "path/to/curious.mp3"},
        {"id": "tension", "path": "path/to/tension.mp3"},
        {"id": "excitement", "path": "path/to/excitement.mp3"}
      ]
    }
  }
}
```

### Slice-Based Control

The application uses **slices** as waypoints that define target emotion ratios. Each slice definition optionally includes emotion parameters and transition settings:

```javascript
{
  id: "scene_01",
  image: "path/to/image.jpg",
  audio: {
    music: {
      transitionDuration: 3000, // ms - crossfade duration
      tracks: [
        {
          "id": "calmness",
          "volume": 0.8
        },
        {
          "id": "curiousness",
          "volume": 0.2
        },
        {
          "id": "tension",
          "volume": 0.0
        },
        {
          "id": "excitement",
          "volume": 0.0
        }
      ]
    }
  }
}
```
