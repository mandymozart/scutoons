# Label-Based Navigation Architecture

## Overview

The chapter rendering system uses **label-based navigation** instead of position-based rendering. This creates a single-dimensional, clean architecture.

## Core Principles

### 1. **Strict Groups**
- Everything is a group (enforced by sliceNormalizer)
- Groups are the atomic rendering unit
- Each group has a label

### 2. **Label-Based Navigation**
- Navigate by label, not by array index
- `ChapterView` builds a label→group map
- All jumps use `renderFromLabel(label)`

### 3. **Explicit Flow Control**
- Interactions MUST end with explicit `jumpTo` or `end` event
- No implicit continuation
- Missing `jumpTo` triggers configuration error alert

### 4. **Auto-Stitching Non-Interactive Groups**
- Non-interactive groups are stitched together
- Rendering stops when an interactive group is hit
- After interaction completes, jumps to specified label

---

## Architecture Flow

```
Chapter Load
  ↓
Build label→group map
  ↓
renderFromLabel(startLabel)
  ↓
Collect consecutive non-interactive groups
  ↓
IF interaction found → run interaction
  ↓
Interaction completes with jumpTo
  ↓
renderFromLabel(jumpTo)
  ↓
Repeat...
```

---

## Example Structure

### chapter6.json
```json
{
  "slices": [
    {
      "group": {
        "label": "intro",
        "renderer": { "type": "vertical" },
        "slices": [
          { "image": "slice1.jpg" },
          { "image": "slice2.jpg" }
        ]
      }
    },
    {
      "group": {
        "label": "vn-cathedral",
        "renderer": { "type": "vertical" },
        "interaction": "visual-novel",
        "slices": [
          { "scene": "scene-200" }
        ]
      }
    },
    {
      "group": {
        "label": "scene-300",
        "renderer": { "type": "vertical" },
        "slices": [
          { "image": "slice15.jpg" }
        ]
      }
    }
  ]
}
```

### chapter6/scene-200.json
```json
{
  "id": "scene-200",
  "events": [
    { "type": "showMessage", "params": {...} },
    { "type": "showChoices", "params": {...} },
    {
      "type": "end",
      "params": {
        "jumpTo": "scene-300"
      }
    }
  ],
  "slices": [...]
}
```

---

## Flow Example

```
1. Load Chapter 6
2. renderFromLabel("intro")
   - Collect non-interactive groups starting from "intro"
   - Stitch: intro slices
   - Render all stitched slices
   - Hit interaction group → stop collecting

3. User scrolls to end
   - (Future: auto-trigger next labeled section)

4. Manual or automatic: renderFromLabel("vn-cathedral")
   - First group has interaction
   - Run VisualNovelInteraction
   - Load scene-200.json
   - Process events
   - User makes choices
   - Event "end" with jumpTo: "scene-300"
   - onComplete("scene-300")

5. renderFromLabel("scene-300")
   - Collect non-interactive groups from "scene-300"
   - Render remaining slices
   - End of chapter
```

---

## Key Methods

### ChapterView

```javascript
buildGroupMap(normalizedSlices)
  // Creates label → group lookup map

renderFromLabel(startLabel, chapterData, sequenceItems)
  // Main rendering method
  // Collects and stitches non-interactive groups
  // Runs interactions

renderGroupInteraction(groupSlice, chapterData, sequenceItems)
  // Handles interaction groups
  // Sets onComplete callback with jumpTo

jumpToLabel(label, sequenceItems)
  // Updates URL and calls renderFromLabel
```

### VisualNovelInteraction

```javascript
processEvents()
  // Process events sequentially
  // Store jumpToLabel when "end" event or choice.onSelect.jumpTo
  // Call onComplete(jumpToLabel) when done

handleEvent(event)
  // Handle event types: showMessage, joinScene, leaveScene,
  // addChoice, showChoices, end
```

---

## Event Types

### Visual Novel Events

- `showMessage` - Display dialogue
- `joinScene` - Add character to scene
- `leaveScene` - Remove character from scene
- `addChoice` - Add choice option
- `showChoices` - Display choices and wait for selection
- `end` - Explicit end with jumpTo target

### Choice onSelect Actions

```json
{
  "onSelect": {
    "set": { "variable": "name", "value": "val" },
    "add": { "variable": "count", "value": 1 },
    "jumpTo": "label-name"
  }
}
```

---

## Benefits

✅ **Single dimension** - label-based, not position-based  
✅ **Clean separation** - groups don't leak into each other  
✅ **Explicit flow** - all jumps are intentional  
✅ **Modular** - groups are independent units  
✅ **Maintainable** - easy to understand and debug  
✅ **Extensible** - easy to add new interaction types  

---

## Configuration Errors

### Missing jumpTo
```
⚠️ Interaction ended without jumpTo label
```
Alert shown when:
- Visual novel completes without `end` event
- Visual novel completes without choice with `jumpTo`

**Fix:** Add explicit `end` event:
```json
{
  "type": "end",
  "params": { "jumpTo": "next-label" }
}
```

### Label not found
```
❌ Label not found: unknown-label
```
Logged when:
- jumpTo references non-existent label

**Fix:** Ensure all jumpTo targets exist as group labels

---

## Migration from Old Architecture

### Old (Position-Based)
```javascript
// Render all slices
// Track current position
// Append after interaction
// Complex state management
```

### New (Label-Based)
```javascript
// Build label map
// Render from label
// Stitch non-interactive
// Jump to label
// Clean state
```

**Key Changes:**
- Remove append mode
- Remove position tracking
- Add label→group map
- Add auto-stitching
- Add explicit flow control

---

## Future Enhancements

- Auto-trigger next labeled section on scroll-to-end
- Group-level renderer switching (vertical → horizontal within chapter)
- Parallel group loading (prefetch next labeled section)
- Save/load state by label (bookmark system)
- Analytics per labeled section
