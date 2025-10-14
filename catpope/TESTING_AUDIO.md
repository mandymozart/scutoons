# Testing Audio System - Chapter 1

## ✅ Setup Complete

The audio system has been configured for Chapter 1 with a minimal test implementation.

## 📁 Audio Files Used

### Music Tracks
- **`s1c1-instrumental-brass-morton.mp3`** - Tension track (ID: "tension")
- **`s1c2-instrumental.mp3`** - Calmness track (ID: "calmness") - **Default track**

### Sample Triggers
- **`s1c1-bells.mp3`** - Triggers on Slice 1
- **`s1c1-choir-once-it-means-nothing.mp3`** - Triggers on Slice 10
- **`s1c1-choir-loud-loud-loud.mp3`** - Triggers on Slice 20

## 🧪 How to Test

### 1. Open the Chapter
```
http://localhost:8000/catpope/index.html#chapter=1
```
(Or however you're serving the files)

### 2. Open Browser Console
Press **F12** to open Developer Tools and view the Console tab.

### 3. Expected Console Messages

When the chapter loads, you should see:
```
Initializing audio system...
Loading music tracks...
Track loaded: tension
Track loaded: calmness
Loaded 2 music tracks (default: calmness)
Adaptive Music Player initialized
Loading sample library...
Sample loaded: bells
Sample loaded: choir_once
Sample loaded: choir_loud
Loaded 3 samples
Sample Trigger Manager initialized
AudioManager initialized successfully
Observing 94 slices for audio triggers
Audio controls initialized
```

### 4. Start Audio
**Click anywhere or scroll** - This unlocks audio (browser autoplay policy).

You should see:
```
Music started
```

The **calmness** track (s1c2-instrumental.mp3) should start playing at full volume and loop continuously. The **tension** track is also playing but at 0 volume (ready for crossfading).

### 5. Test Sample Triggers

Scroll through the chapter and watch/listen for:

- **Slice 1**: Bells sound should play when slice comes into view
- **Slice 10**: Choir "once it means nothing" should play
- **Slice 20**: Choir "loud loud loud" should play

Each sample plays once when its slice becomes 50% visible.

### 6. Test Adaptive Music (Optional Advanced Test)

To test the crossfading between the two tracks, add volume config to a slice. For example, edit `chapter1.json` and add to slice 30:

```json
{
    "image": "assets/images/3x/de/S1C1 - Slice 30.jpg",
    "audio": {
        "music": {
            "transitionDuration": 3000,
            "tracks": [
                {"id": "calmness", "volume": 0.3},
                {"id": "tension", "volume": 0.7}
            ]
        }
    }
}
```

When you scroll to slice 30, you should hear the music crossfade from calm to tense over 3 seconds!

### 7. Test Audio Controls

Look for floating controls in the **bottom-right corner**.

**Test volume slider**:
- Drag slider left/right
- Verify music and samples adjust volume

**Test mute button**:
- Click speaker icon
- Audio should mute (icon changes to muted state)
- Click again to unmute

**Test keyboard shortcut**:
- Press **M** key
- Should toggle mute/unmute

## 🔍 Troubleshooting

### Audio doesn't play
- **Check**: Did you click or scroll first? (Required for autoplay policy)
- **Check**: Browser console for errors
- **Check**: Audio files are accessible (check Network tab in DevTools)

### Music plays but samples don't
- **Check**: Console for "Sample loaded" messages
- **Check**: You scrolled past slices 1, 10, and 20
- **Check**: Samples aren't too quiet (adjust volume in JSON)

### Audio controls don't appear
- **Check**: Console for errors
- **Check**: CSS is loaded (`catpope/assets/styles.css`)
- **Check**: Audio config exists in chapter1.json

### Console errors
- **"Failed to load"**: Check file paths in chapter1.json
- **"Howler not defined"**: Check Howler.js CDN loaded in index.html
- **"AudioManager already initialized"**: Normal if navigating between views

## 📊 What's Happening

### On Chapter Load
1. ChapterView detects `audio` config in chapter1.json
2. AudioManager initializes
3. AdaptiveMusicPlayer loads music track
4. SampleTriggerManager loads all samples
5. IntersectionObserver watches all 94 slices

### On First User Interaction
1. Music starts playing at full volume
2. Music loops continuously

### On Scroll
1. When Slice 1 is 50% visible → bells trigger
2. When Slice 10 is 50% visible → choir "once" triggers
3. When Slice 20 is 50% visible → choir "loud" triggers
4. Each sample only triggers once per view

## 🎛️ Adjusting the Configuration

### Change Sample Volumes
Edit `chapter1.json`:
```json
"volume": 0.8  // 0.0 = silent, 1.0 = full volume
```

### Add Delay to Samples
```json
"delay": 500  // milliseconds
```

### Add More Sample Triggers
Copy a slice with audio config and change the slice number:
```json
{
    "image": "assets/images/3x/de/S1C1 - Slice 30.jpg",
    "audio": {
        "samples": [
            {
                "id": "bells",
                "trigger": {"type": "scrollInView", "mode": "mono"},
                "volume": 0.5,
                "delay": 0
            }
        ]
    }
}
```

### Try Different Trigger Types
```json
"trigger": {"type": "click", "mode": "poly"}  // Click to play
"trigger": {"type": "hover", "mode": "mono"}  // Hover to play
```

### Test Adaptive Music (Advanced)
To test the full 4-track system, you'd need 4 music files and add volume controls to slices:

```json
"audio": {
    "music": {
        "tracks": [
            {"id": "calm", "path": "assets/audio/music/calm.mp3"},
            {"id": "tense", "path": "assets/audio/music/tense.mp3"}
        ]
    }
}

// In slice:
"audio": {
    "music": {
        "tracks": [
            {"id": "calm", "volume": 0.8},
            {"id": "tense", "volume": 0.2}
        ]
    }
}
```

## ✅ Success Criteria

The audio system is working correctly if:
- [x] Background music starts on first interaction
- [x] Music loops continuously
- [x] Bells sound plays on Slice 1
- [x] Choir sounds play on Slices 10 and 20
- [x] Audio controls appear and function
- [x] Mute button works
- [x] Volume slider adjusts audio
- [x] No console errors
- [x] M key toggles mute

## 📝 Next Steps

Once basic testing works:

1. **Adjust volumes** to taste
2. **Add more sample triggers** to other slices
3. **Try different trigger types** (click, hover)
4. **Test on mobile** devices
5. **Create adaptive music** with multiple tracks
6. **Add custom trigger rects** for interactive hotspots

## 🐛 Known Limitations (Current Implementation)

- **Envelope not implemented**: Attack/sustain/release config exists but doesn't affect playback yet
- **Pan not implemented**: Stereo positioning config exists but doesn't work yet
- **Pitch not implemented**: Pitch shift config exists but doesn't work yet
- **Exclusive mode not implemented**: Would mute other audio when triggered

These can be added later if needed. Basic functionality (volume, delay, trigger modes) works perfectly.

## 📞 Need Help?

Check the full documentation:
- **Quick Reference**: `issues/plans/AUDIO_QUICK_REFERENCE.md`
- **API Docs**: `catpope/app/audio/README.md`
- **Implementation Plan**: `issues/plans/1-music-and-audio.md`
