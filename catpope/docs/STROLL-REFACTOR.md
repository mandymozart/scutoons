# Stroll Interaction Refactoring

## Overview

Refactored stroll interactions to use the new groups-based architecture with external group files (`GroupRef`).

---

## What Changed

### Before (Old Format)
```json
{
  "groups": [
    {
      "label": "scene-13-blue-hour",
      "renderer": { "type": "vertical" },
      "interaction": "stroll",
      "slices": [
        { "scene": "scene-14" },
        { "scene": "scene-15" },
        { "scene": "scene-16" }
      ]
    }
  ]
}
```

**Problems:**
- ❌ Nested scene references don't match `Group` type
- ❌ Scene files had `id` instead of `label`
- ❌ Scene files missing required `renderer` and `interaction` fields

### After (New Format)

**chapter2.json:**
```json
{
  "groups": [
    { "src": "chapter2/scene-14.json" },
    { "src": "chapter2/scene-15.json" },
    { "src": "chapter2/scene-16.json" },
    {
      "label": "scene-20",
      "renderer": { "type": "vertical" },
      "slices": [...]
    }
  ]
}
```

**chapter2/scene-14.json:**
```json
{
  "label": "scene-14",
  "renderer": { "type": "vertical" },
  "interaction": "stroll",
  "slices": [
    { "image": "images/1x/de/s1c2-slice-1.jpg" },
    { "image": "images/1x/de/s1c2-slice-2.jpg" }
  ]
}
```

---

## How Stroll Works Now

### 1. **External Group Files**
Each stroll scene is a separate Group JSON file:
- ✅ `label` - Scene identifier
- ✅ `renderer` - Required renderer type
- ✅ `interaction: "stroll"` - Marks as stroll scene
- ✅ `slices` - Actual image slices

### 2. **GroupRef References**
Chapter file uses `GroupRef` to reference external files:
```json
{ "src": "chapter2/scene-14.json" }
```

### 3. **Consecutive Group Collection**
When ChapterView encounters the first stroll group:
1. Detects `interaction: "stroll"`
2. Calls `collectInteractionGroups()` to collect ALL consecutive stroll groups
3. Passes their labels as scenes array to `StrollInteraction`
4. StrollInteraction randomizes between them

### 4. **StrollInteraction Flow**
```
1. ChapterView finds first stroll group (scene-14)
2. Collects all consecutive stroll groups (scene-14 through scene-19)
3. Extracts labels: ["scene-14", "scene-15", ...]
4. Passes to StrollInteraction
5. StrollInteraction picks random scene
6. Loads scene data using label
7. Renders slices
8. User interacts (time progression)
9. On complete: jumpTo next label or continue to next group
```

---

## Code Changes

### 1. **GroupUtils.js**
Added `collectInteractionGroups()`:
```javascript
/**
 * Collect consecutive groups with the same interaction type
 * Used for stroll where multiple groups are part of one interaction
 */
export function collectInteractionGroups(groups, startIndex, interactionType) {
  const collectedGroups = [];
  let currentIndex = startIndex;

  while (currentIndex < groups.length) {
    const group = groups[currentIndex];
    if (group.interaction !== interactionType) break;
    collectedGroups.push(group);
    currentIndex++;
  }

  return {
    groups: collectedGroups,
    nextIndex: currentIndex < groups.length ? currentIndex : null
  };
}
```

### 2. **ChapterView.js**
Added `renderStrollInteraction()`:
```javascript
async renderStrollInteraction(strollGroups, chapterData, sequenceItems, nextIndex) {
  // Extract scene labels from groups
  const scenes = strollGroups.map(g => g.label);
  
  // Create stroll interaction with all scenes
  const interaction = this.interactionManager.create(
    'stroll',
    {},
    scenes, // ["scene-14", "scene-15", ...]
    renderer
  );
  
  interaction.onComplete = async (jumpToLabel) => {
    if (jumpToLabel) {
      await this.renderFromLabel(jumpToLabel, chapterData, sequenceItems);
    } else if (nextIndex !== null) {
      // Continue with next group after stroll
      await this.renderFromLabel(this.groups[nextIndex].label, chapterData, sequenceItems);
    }
  };
  
  await interaction.init();
}
```

Updated `renderFromLabel()`:
```javascript
// Special case: stroll interactions collect ALL consecutive stroll groups
if (interactionGroup.interaction === 'stroll') {
  const { groups: strollGroups, nextIndex } = collectInteractionGroups(
    this.groups,
    nextInteractionIndex,
    'stroll'
  );
  await this.renderStrollInteraction(strollGroups, chapterData, sequenceItems, nextIndex);
} else {
  await this.renderGroupInteraction(interactionGroup, chapterData, sequenceItems);
}
```

### 3. **Scene Files**
Converted all `chapter2/scene-*.json` files:
```javascript
// Before
{
  "id": "scene-14",
  "slices": [...]
}

// After
{
  "label": "scene-14",
  "renderer": { "type": "vertical" },
  "interaction": "stroll",
  "slices": [...]
}
```

---

## Benefits

### Type Safety
✅ All groups conform to `Group` interface  
✅ No special `scene` references  
✅ Required fields enforced  

### Modularity
✅ Each scene is a separate file  
✅ Easy to add/remove/reorder scenes  
✅ Scenes can be reused across chapters  

### Flexibility
✅ Scenes can have different renderers (future)  
✅ Scenes can define their own audio/config  
✅ Clean separation of concerns  

### Consistency
✅ Same `GroupRef` pattern as visual novel  
✅ All groups follow same structure  
✅ No special cases in type definitions  

---

## Example: Chapter 2

**File Structure:**
```
api/static/catpope/de/
├── chapter2.json           # Main chapter (GroupRef array)
└── chapter2/
    ├── scene-14.json      # Stroll group
    ├── scene-15.json      # Stroll group
    ├── scene-16.json      # Stroll group
    ├── scene-17.json      # Stroll group
    ├── scene-18.json      # Stroll group
    └── scene-19.json      # Stroll group
```

**Runtime Flow:**
1. Load `chapter2.json`
2. `populateGroups()` loads all `GroupRef` files
3. ChapterView starts rendering from first label
4. Encounters `scene-14` with `interaction: "stroll"`
5. Collects `scene-14` through `scene-19` (all consecutive stroll groups)
6. Passes `["scene-14", "scene-15", ...]` to StrollInteraction
7. StrollInteraction randomly picks and displays scenes
8. User progresses through time
9. On complete: jumps to `scene-20` (next group)

---

## Migration Guide

### Converting Old Stroll Format

**Step 1: Extract scene files**
```bash
# Each scene file gets:
- label (from id)
- renderer: "vertical"
- interaction: "stroll"
- slices (existing)
```

**Step 2: Update chapter file**
```json
// Remove wrapper group
{
  "groups": [
    // OLD: wrapper with scene references
    {
      "label": "...",
      "interaction": "stroll",
      "slices": [{ "scene": "..." }]
    }
    
    // NEW: direct GroupRef
    { "src": "chapter2/scene-14.json" },
    { "src": "chapter2/scene-15.json" }
  ]
}
```

**Step 3: No code changes needed**
ChapterView automatically detects and handles consecutive stroll groups!

---

## Future Enhancements

### Different Renderers per Scene
```json
{
  "label": "scene-14",
  "renderer": { "type": "vertical" },    // Most scenes vertical
  "interaction": "stroll",
  "slices": [...]
}

{
  "label": "scene-15",
  "renderer": "horizontal",  // Special scene horizontal!
  "interaction": "stroll",
  "slices": [...]
}
```

### Scene-Specific Audio
```json
{
  "label": "scene-14",
  "renderer": { "type": "vertical" },
  "interaction": "stroll",
  "audio": {
    "music": {
      "tracks": [...]
    }
  },
  "slices": [...]
}
```

### Mixed Interactions
```json
{
  "groups": [
    { "src": "stroll-morning.json" },  // stroll
    { "src": "stroll-noon.json" },     // stroll
    { "src": "dialogue-13.json" },     // visual-novel
    { "src": "stroll-evening.json" },  // stroll (new sequence)
    { "src": "stroll-night.json" }     // stroll
  ]
}
```

Automatically groups:
- stroll-morning + stroll-noon → one stroll interaction
- dialogue-13 → visual-novel interaction
- stroll-evening + stroll-night → another stroll interaction

---

## Summary

**Removed:**
- ❌ Wrapper groups with scene references
- ❌ `{ "scene": "..." }` slice format
- ❌ `id` field in scene files

**Added:**
- ✅ `GroupRef` for external scene files
- ✅ Required `label`, `renderer`, `interaction` in scene files
- ✅ `collectInteractionGroups()` utility
- ✅ `renderStrollInteraction()` method

**Result:**
- Clean, type-safe architecture
- Modular scene files
- Flexible interaction grouping
- No special cases in type system

All stroll interactions now conform to the standard groups-based architecture!
