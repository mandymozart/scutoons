# Audio System Migration Guide

## Overview

The SCUTOONS audio system has been updated to integrate seamlessly with Svelte 5's reactivity system while maintaining the AudioManager's framework independence.

## What Changed

### ✅ Kept (Framework-Agnostic)
- `AudioManager` - Core coordinator
- `AdaptiveMusicPlayer` - Music playback logic
- `SampleTriggerManager` - Sample loading and triggering
- All audio types and interfaces

### 🆕 Added (Svelte Integration)
- `contexts/audio.svelte.ts` - Svelte context wrapper with lifecycle management
- `actions/audio.ts` - Svelte actions for declarative DOM triggers

### 🗑️ Removed (DOM-Coupled Code)
- `SampleTriggerManager.registerSliceTriggers()` - Moved to Svelte actions
- `SampleTriggerManager.registerTrigger()` - Moved to Svelte actions
- `SampleTriggerManager.registerintersectTrigger()` - Moved to Svelte actions
- `SampleTriggerManager.registerClickTrigger()` - Moved to Svelte actions
- `SampleTriggerManager.registerHoverTrigger()` - Moved to Svelte actions
- `SampleTriggerManager.isWithinRect()` - Moved to Svelte actions
- `activeTriggers` state - Now handled by Svelte action lifecycle
- `intersectionObserver` state - Now handled by Svelte action lifecycle

## Migration Steps

### Before (Old Imperative Approach)
```javascript
// Somewhere in your component
const audioManager = new AudioManager();
await audioManager.init(chapterAudio);

const sampleManager = audioManager.getSampleManager();
const sliceElement = document.querySelector('.slice');
sampleManager.registerSliceTriggers(sliceElement, samples);
```

### After (New Svelte Approach)

#### 1. Layout (`[toonId]/+layout.svelte`)
```svelte
<script lang="ts">
  import { setAudioContext } from '$lib/contexts/audio.svelte.ts';
  import { onDestroy } from 'svelte';

  const audio = setAudioContext();

  onDestroy(async () => {
    await audio.unload();
  });
</script>
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

#### 3. Slice Component
```svelte
<script lang="ts">
  import { audioTrigger } from '$lib/actions/audio.ts';
  
  let { slice } = $props();
</script>

{#if slice.audio?.samples}
  <div use:audioTrigger={slice.audio.samples[0]}>
    <img src={slice.image} />
  </div>
{/if}
```

## Key Concepts

### Context Lifecycle

```
Navigation: /catpope → /catpope/chapter/1 → /catpope/chapter/2
            
Layout      [setAudioContext()]────────────────────────[unload()]
                    │                                        
Chapter 1           └─[loadConfiguration(A)]──[unload()]
                                │                  
Chapter 2                       └─────────────[loadConfiguration(B)]
```

### Audio Triggers

| Trigger Type | Before | After |
|--------------|--------|-------|
| Scroll | `registerintersectTrigger()` | `use:audioTrigger` |
| Click | `registerClickTrigger()` | `use:audioTrigger` |
| Hover | `registerHoverTrigger()` | `use:audioTrigger` |
| Manual | `trigger(sample)` | `audio.triggerSample(sample)` |

### Reactive State

```svelte
<script>
  const audio = getAudioContext();
  
  // Reactive access
  $effect(() => {
    console.log('Muted:', audio.state.isMuted);
    console.log('Playing:', audio.state.isPlaying);
    console.log('Volume:', audio.state.masterVolume);
  });
</script>

<button onclick={() => audio.toggleMute()}>
  {audio.state.isMuted ? 'Unmute' : 'Mute'}
</button>
```

## Benefits of New Approach

### ✅ Svelte-Native
- Uses `$state` for reactivity
- Uses `$effect` for lifecycle
- Uses actions for DOM integration
- Automatic cleanup on unmount

### ✅ Per-Chapter Configuration
- Each chapter loads its own audio config
- Smooth transitions between chapters
- Graceful fade-out on navigation

### ✅ Declarative API
```svelte
<!-- Old: Imperative -->
<script>
  onMount(() => {
    const cleanup = registerTrigger(element, config);
    return cleanup;
  });
</script>

<!-- New: Declarative -->
<div use:audioTrigger={config}>
```

### ✅ TypeScript Support
- Full type safety
- IntelliSense for audio context
- Compile-time checks

## Troubleshooting

### "Audio context not available"
**Cause**: Trying to trigger audio before context is set  
**Fix**: Ensure `setAudioContext()` is called in parent layout

### "Cannot read property 'loadConfiguration' of undefined"
**Cause**: Using `getAudioContext()` outside of Svelte component tree  
**Fix**: Use `getCurrentAudioContext()` for non-component access

### Audio doesn't clean up between chapters
**Cause**: Missing cleanup in `$effect` return  
**Fix**: Add `return () => { untrack(() => audio.unload()); }`

### Samples trigger multiple times
**Cause**: Svelte action re-running  
**Fix**: This is expected - the action cleanup prevents duplicates

## Testing

```svelte
<script lang="ts">
  const audio = getAudioContext();

  // Debug audio state
  $effect(() => {
    console.log('🎵 Audio State:', {
      isInitialized: audio.state.isInitialized,
      isPlaying: audio.state.isPlaying,
      hasConfig: !!audio.state.currentConfig
    });
  });
</script>
```

## Next Steps

1. ✅ Remove old DOM manipulation code from components
2. ✅ Add `use:audioTrigger` to slice components
3. ✅ Test chapter-to-chapter transitions
4. ⏳ Implement visual novel event audio triggers
5. ⏳ Add audio controls UI component
6. ⏳ Test on mobile devices

## Questions?

See:
- `lib/audio/README.md` - Full documentation
- `lib/contexts/audio.svelte.ts` - Context implementation
- `lib/actions/audio.ts` - Action implementation
