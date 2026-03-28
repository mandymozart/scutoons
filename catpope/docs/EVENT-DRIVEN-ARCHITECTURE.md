# Event-Driven Architecture for Interactions

## Overview

Refactored visual novel (and future interactions) to use **event-driven architecture** instead of separate interaction classes. This eliminates code duplication and simplifies the delegation between ChapterView, renderers, and interactions.

---

## Core Concept

**Groups with `events` array process events sequentially.**

```json
{
  "label": "dialogue-1",
  "renderer": { "type": "vertical" },
  "events": [
    {
      "type": "showMessage",
      "params": {
        "text": "Hello, stranger.",
        "character": "PLF"
      }
    },
    {
      "type": "end",
      "params": {
        "jumpTo": "next-scene"
      }
    }
  ],
  "slices": [...]  // Optional background images
}
```

---

## Architecture

### 1. ChapterView → Flow Coordinator
**Responsibility:** Detect and dispatch

```javascript
async renderFromLabel(label) {
  const group = this.groupMap.get(label);
  
  // NEW: Check if group has events
  if (group.events && group.events.length > 0) {
    await this.processGroupEvents(group, chapterData, sequenceItems);
    return;
  }
  
  // OLD: Legacy interaction system (stroll)
  if (group.interaction === 'stroll') {
    await this.renderStrollInteraction(...);
    return;
  }
  
  // DEFAULT: Just render slices
  await this.renderGroupSlices(group);
}

async processGroupEvents(group, chapterData, sequenceItems) {
  // Render background slices first
  if (group.slices && group.slices.length > 0) {
    await this.renderGroupSlices(group);
  }
  
  // Process events
  const eventProcessor = new EventProcessor(this.scutoon, group);
  const nextLabel = await eventProcessor.run();
  eventProcessor.cleanup();
  
  // Jump to next label if specified
  if (nextLabel) {
    await this.renderFromLabel(nextLabel, chapterData, sequenceItems);
  }
}
```

---

### 2. EventProcessor → Event Dispatcher
**Responsibility:** Run events in sequence

**File:** `app/controllers/EventProcessor.js`

```javascript
export class EventProcessor {
  constructor(scutoon, group) {
    this.scutoon = scutoon;
    this.group = group;
    this.events = group.events || [];
    
    // Event modules
    this.vnEvents = new VisualNovelEvents(scutoon);
  }

  async run() {
    for (const event of this.events) {
      const result = await this.processEvent(event);
      
      // If event returns jumpTo, exit early
      if (result?.jumpTo) {
        return result.jumpTo;
      }
    }
    return null;
  }

  async processEvent(event) {
    switch (event.type) {
      case 'showMessage':
        return await this.vnEvents.showMessage(event.params);
      case 'showChoices':
        return await this.vnEvents.showChoices(event.params);
      case 'addChoice':
        return await this.vnEvents.addChoice(event.params);
      case 'joinScene':
        return await this.vnEvents.joinScene(event.params);
      case 'leaveScene':
        return await this.vnEvents.leaveScene(event.params);
      case 'end':
        return { jumpTo: event.params.jumpTo };
      default:
        console.warn(`Unknown event type: ${event.type}`);
    }
  }
}
```

---

### 3. VisualNovelEvents → Event Handlers
**Responsibility:** Handle specific event types

**File:** `app/controllers/events/VisualNovelEvents.js`

```javascript
export class VisualNovelEvents {
  constructor(scutoon) {
    this.scutoon = scutoon;
    this.messageBox = null;
    this.choicesContainer = null;
    this.charactersContainer = null;
    this.characters = new Map();
  }

  async showMessage({ text, character, wait = true }) {
    // Build UI if needed
    if (!this.messageBox) this.buildUI();
    
    // Update text
    this.messageBox.querySelector('.vn-message-text').textContent = text;
    this.messageBox.querySelector('.vn-character-name').textContent = character;
    
    // Wait for user click
    if (wait) {
      return new Promise(resolve => {
        const clickHandler = () => {
          this.messageBox.removeEventListener('click', clickHandler);
          resolve();
        };
        this.messageBox.addEventListener('click', clickHandler);
      });
    }
  }

  async addChoice({ label, onSelect }) {
    const button = document.createElement('button');
    button.textContent = label;
    
    return new Promise(resolve => {
      button.addEventListener('click', () => {
        // Handle variable modification
        if (onSelect.set) {
          this.scutoon.gameStore.set(onSelect.set.variable, onSelect.set.value);
        } else if (onSelect.add) {
          const current = this.scutoon.gameStore.get(onSelect.add.variable) || 0;
          this.scutoon.gameStore.set(onSelect.add.variable, current + onSelect.add.value);
        }
        
        this.choicesContainer.style.display = 'none';
        resolve();
      });
      
      this.choicesContainer.appendChild(button);
    });
  }

  async joinScene({ characters }) {
    // Add characters to stage
  }

  async leaveScene({ characters }) {
    // Remove characters from stage
  }

  buildUI() {
    // Create message box, choices, characters container
  }

  cleanup() {
    // Remove UI elements
  }
}
```

---

## Event Types

### Visual Novel Events

#### `showMessage`
Display message with character name

```json
{
  "type": "showMessage",
  "params": {
    "text": "Hello, stranger.",
    "character": "PLF",
    "wait": true
  }
}
```

#### `showChoices`
Prepare choices UI (clears previous)

```json
{
  "type": "showChoices",
  "params": {}
}
```

#### `addChoice`
Add a choice button with variable modification

```json
{
  "type": "addChoice",
  "params": {
    "label": "Be friendly",
    "onSelect": {
      "set": {
        "variable": "relationship_plf",
        "value": "friendly"
      }
    }
  }
}
```

```json
{
  "type": "addChoice",
  "params": {
    "label": "Be suspicious",
    "onSelect": {
      "add": {
        "variable": "suspicion_level",
        "value": 10
      }
    }
  }
}
```

#### `joinScene`
Add characters to scene

```json
{
  "type": "joinScene",
  "params": {
    "characters": [
      {
        "name": "PLF",
        "position": {
          "variant": "full",
          "relativeToMessageBox": "back",
          "order": 0
        },
        "mood": "happy"
      }
    ]
  }
}
```

#### `leaveScene`
Remove characters from scene

```json
{
  "type": "leaveScene",
  "params": {
    "characters": [
      { "name": "PLF" }
    ]
  }
}
```

#### `end`
End events and jump to label

```json
{
  "type": "end",
  "params": {
    "jumpTo": "scene-300"
  }
}
```

---

## Complete Example

**chapter6/scene-200.json:**
```json
{
  "label": "scene-200",
  "renderer": { "type": "vertical" },
  "events": [
    {
      "type": "joinScene",
      "params": {
        "characters": [
          {
            "name": "PLF",
            "position": {
              "variant": "full",
              "relativeToMessageBox": "back",
              "order": 0
            },
            "mood": "neutral"
          }
        ]
      }
    },
    {
      "type": "showMessage",
      "params": {
        "text": "What a beautiful day!",
        "character": "PLF"
      }
    },
    {
      "type": "showChoices",
      "params": {}
    },
    {
      "type": "addChoice",
      "params": {
        "label": "Agree",
        "onSelect": {
          "add": {
            "variable": "relationship_plf",
            "value": 5
          }
        }
      }
    },
    {
      "type": "addChoice",
      "params": {
        "label": "Disagree",
        "onSelect": {
          "add": {
            "variable": "relationship_plf",
            "value": -5
          }
        }
      }
    },
    {
      "type": "showMessage",
      "params": {
        "text": "Let's continue...",
        "character": "PLF"
      }
    },
    {
      "type": "leaveScene",
      "params": {
        "characters": [{ "name": "PLF" }]
      }
    },
    {
      "type": "end",
      "params": {
        "jumpTo": "scene-300"
      }
    }
  ],
  "slices": [
    { "image": "images/background-cathedral.jpg" }
  ]
}
```

---

## Benefits

### ✅ No Code Duplication
- Message showing → One handler (`showMessage`)
- Character management → One handler (`joinScene`/`leaveScene`)
- Variable management → Unified in choice handler

### ✅ Clear Separation
- **ChapterView** → Coordinate flow
- **EventProcessor** → Dispatch events
- **VisualNovelEvents** → Handle VN events
- **Renderers** → Just render HTML

### ✅ Easy to Extend
Add new event type:
1. Add to `types.d.ts`
2. Add handler in `VisualNovelEvents`
3. Add case in `EventProcessor.processEvent()`

### ✅ Data-Driven
JSON defines behavior, not code classes

### ✅ Testable
Each module can be tested independently

---

## Migration Path

### Old (Interaction Class)
```javascript
// VisualNovelInteraction.js
class VisualNovelInteraction {
  constructor(scutoon, config, scenes, renderer) {}
  async init() {
    await this.loadScene();
    await this.renderBackground();
    await this.processEvents();
  }
  async processEvents() {
    for (const event of this.events) {
      await this.handleEvent(event);
    }
  }
}
```

### New (Event-Driven)
```json
{
  "label": "scene-200",
  "renderer": { "type": "vertical" },
  "events": [...],
  "slices": [...]
}
```

**No interaction class needed!**
Events are processed by `EventProcessor` → `VisualNovelEvents`

---

## Future: Stroll Migration

**Stroll will eventually become events too:**

```json
{
  "label": "morning-stroll",
  "renderer": { "type": "vertical" },
  "events": [
    {
      "type": "timePicker",
      "params": {
        "startTime": "09:00",
        "endTime": "09:45",
        "increment": 15
      }
    },
    {
      "type": "randomJump",
      "params": {
        "targets": ["scene-14", "scene-15", "scene-16"]
      }
    },
    {
      "type": "end",
      "params": {
        "jumpTo": "scene-20"
      }
    }
  ]
}
```

---

## Summary

### Removed:
- ❌ `VisualNovelInteraction` class
- ❌ Separate interaction lifecycle management
- ❌ Code duplication for message/choice/character handling

### Added:
- ✅ `EventProcessor` - Universal event dispatcher
- ✅ `VisualNovelEvents` - Clean event handlers
- ✅ Event-driven JSON format
- ✅ `processGroupEvents()` in ChapterView

### Result:
- **Less code** - No interaction classes needed
- **More flexible** - Add events without new classes
- **Better organized** - Clear separation of concerns
- **Easier to maintain** - One place for each event type

**Visual novel now works through pure event processing!** 🎉
