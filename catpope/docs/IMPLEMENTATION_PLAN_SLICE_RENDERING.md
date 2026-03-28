# Slice Rendering System - Implementation Plan

## Confirmed Specifications

### 1. Interaction Types
- **Default renderer**: Seamless scrolling (no interaction required)
- **Stroll**: Random scene pick with timer, exclusive view
- **Choice**: Rudimentary branching (basic implementation)
- **Quest**: Rudimentary sub-quest system (basic implementation)

### 2. Scene Loading
- **Eager loading**: Load all scenes when chapter loads
- Future: Implement careful lazy loading strategy

### 3. Stroll Interaction Behavior
- Random scene selection
- Timer counting up with each click (15-minute game time intervals)
- Track checkpoint history: Season > Chapter > Scene > Slice
- Future: Free navigation between scenes

### 4. State Persistence
- **localStorage** for now
- Future: MySQL database backend

### 5. Rendering Approach
- **No visual wrapper** for default renderer (seamless, virtual grouping)
- **Exclusive view** for interactions (stroll/quest/choice) - like ChapterView
- ChapterView handles delegation
- Renderers load their own views when applicable
- **URL state reflection**: chapter, season, interaction state

## Architecture

### Renderer Behavior
- **Default renderer + no interaction** → Seamless scrolling
- **Default renderer + interaction specified** → Exclusive interaction view
- ChapterView delegates based on slice type/interaction
- Not overlay - full view replacement

### Example Flow (from chapter2.json)
```
Slices 1-15:    Group, no special renderer → Default scroll
Slices 16-24:   No group → Default from chapter.json
Slices 25-29:   Group, stroll interaction → Exclusive stroll view
Slices 30-34:   Group, stroll interaction → Exclusive stroll view
Slices 35-39:   Group, stroll interaction → Exclusive stroll view
Exit point:     "scene-19-cathedral-exterior" label
Slices 40-90:   Normal order → Default scroll
```

## Implementation Phases

### Phase 1: Data Processing
**File: `app/utils/sliceNormalizer.js`**
- Process chapter slices recursively
- Expand groups and load scene files
- Merge attributes (group → scene → slice inheritance)
- Track metadata (labels, interaction types, indices)
- Output: normalized slice array with metadata

**File: `app/utils/sceneLoader.js`**
- Load scene JSON from `chapter{n}/` subfolder
- Cache loaded scenes
- Error handling

**File: `app/utils/checkpointTracker.js`**
- Track checkpoint history: Season > Chapter > Scene > Slice
- localStorage persistence
- Query methods (hasReached, getHistory, etc.)

### Phase 2: Interaction System
**File: `app/interactions/InteractionManager.js`**
- Manages interaction lifecycle
- Navigation state (main thread vs interaction view)
- Handles completion → return to main thread
- URL state management

**File: `app/interactions/StrollInteraction.js`**
- Random scene picker
- Timer system (15-minute intervals)
- Checkpoint tracking on each selection
- Exit logic
- Exclusive view rendering

**File: `app/interactions/ChoiceInteraction.js`** (rudimentary)
- Display choices
- Handle selection
- Navigate to chosen path
- Track decision

**File: `app/interactions/QuestInteraction.js`** (rudimentary)
- Display quest info
- Track completion
- Continue flow

### Phase 3: Rendering System
**File: `app/components/SliceRenderer.js`**
- Determine render type from normalized slice
- Check for interaction requirement
- Delegate to default renderer or interaction view
- Handle transitions

**Update: `app/views/ChapterView.js`**
- Import sliceNormalizer
- Process chapter data on load
- Initialize InteractionManager
- Handle view delegation based on current state
- Update URL when entering/exiting interactions

### Phase 4: URL & State
**URL Structure:**
- Main thread: `/chapter/2`
- In stroll: `/chapter/2/stroll/scene-14-alterlaa-church`
- In choice: `/chapter/2/choice/decision-point`
- In quest: `/chapter/2/quest/find-the-key`

**State Management:**
- Current position in main thread
- Current interaction context (if any)
- Checkpoint history
- Timer state

## Data Structure Examples

### Normalized Slice Output
```javascript
{
  originalIndex: 0,
  globalIndex: 0,
  type: 'default' | 'group-member',
  groupLabel: 'scene-13-blue-hour',
  interaction: null | 'stroll' | 'choice' | 'quest',
  exitLabel: 'scene-19-cathedral-exterior',
  image: 'images/...',
  audio: { ... },  // Merged attributes
  // ... other properties
}
```

### Checkpoint Structure
```javascript
{
  season: 1,
  chapter: 2,
  scene: 'scene-14-alterlaa-church',
  slice: 3,
  timestamp: '2025-11-10T12:00:00Z',
  interactionType: 'stroll'
}
```

## Future Enhancements
- **Configuration Inheritance Hierarchy** (TO IMPLEMENT AFTER RENDERERS)
  - Slice-level config overrides (highest priority)
  - Group-level config overrides (medium priority)
  - Chapter-level config (fallback, default)
  - Applies to: audio, interaction params, renderer settings
- Lazy/preload scene loading
- Free navigation in stroll mode
- MySQL backend for progress
- Advanced choice/quest systems
- Scene transition animations
- Checkpoint visualization UI
