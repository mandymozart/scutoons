# Visual Novel Implementation Plan

## Architecture Analysis

### Old App Structure (catpope/)

#### Core Responsibilities

**1. GroupHandler** (`app/controllers/GroupHandler.js`)
- Orchestrates chapter group processing
- Routes to appropriate handlers:
  - Event groups → `processEventGroup()`
  - Interactive groups → `processInteractionGroup()`
  - Webtoon groups → `processWebtoonGroup()`
- Manages renderer lifecycle
- Handles group completion and jump navigation

**2. VisualNovelInteraction** (`app/controllers/interactions/VisualNovelInteraction.js`)
- Initializes VN interaction for a specific group
- Renders background slices
- Creates and runs `EventProcessor` for the group's events
- Provides `onComplete` callback for jump navigation
- Cleanup on completion

**3. EventProcessor** (`app/controllers/events/EventProcessor.js`)
- Processes sequential VN events from a queue
- Handles event types:
  - `showMessage` - Display character dialogue
  - `showChoices` - Show choice container
  - `addChoice` - Add individual choice button
  - `joinScene` - Add characters to scene
  - `leaveScene` - Remove characters from scene
  - `end` - Return jumpTo label for navigation
- Manages UI element references
- Returns `jumpTo` label for group navigation

**4. VisualNovelStore** (`app/stores/VisualNovelStore.js`)
- Loads and caches character data per language
- Provides `getCharacter(name)` lookup
- Manages pending choices array
- Handles choice event handler cleanup via `clearChoices()`

**5. GameStore** (`app/stores/GameStore.js`)
- Persists game state to localStorage
- Manages per-toon save data:
  - `variables` - Player choices, scores (e.g., cucumber count)
  - `progress` - Chapters, scenes, labels completed
  - `lastCheckpoint` - Last saved position
- Provides `get(key)` and `set(key, value)` for variables
- Auto-saves on variable changes with timestamps

#### Component Hierarchy

```
MessageBox.js
├── renderMessageBox() - Returns HTML string
└── handleShowMessage() - Creates element, waits for click if needed

Choices.js
├── renderChoices() - Returns container HTML
└── handleShowChoices() - Shows container, resolves immediately

Choice.js
├── renderChoice() - Returns button HTML
└── handleAddChoice() - Adds choice, waits for click, handles variable changes

Characters.js
├── renderCharacters() - Array of characters to HTML
└── Character.js - Individual character rendering
```

#### Data Flow

1. **Chapter Load** → Groups with `interaction: "visual-novel"`
2. **GroupHandler** detects interaction → creates `VisualNovelInteraction`
3. **VisualNovelInteraction.init()** → renders background + starts EventProcessor
4. **EventProcessor.run()** → processes events sequentially:
   - Each event returns Promise (blocking/non-blocking)
   - `end` event returns `{jumpTo: label}` to exit early
5. **On completion** → `onComplete(jumpToLabel)` called
6. **GroupHandler** jumps to next group or ends chapter

---

## New Svelte App Architecture

### Project Context

- **Framework**: SvelteKit with Svelte 5 runes (`$state`, `$derived`, `$props`)
- **Context Pattern**: `setContext/getContext` (see `audio.svelte.ts`, `membership.svelte.ts`)
- **Existing Structure**:
  - Groups already defined in data with `interaction` and `events` fields
  - Renderer components: `horizontal.svelte`, `vertical.svelte`
  - Type definitions in `models/visual-novel.ts` (already matching old app)

### Implementation Plan

---

## Phase 1: Context & State Management

### 1.1 GameContext (`lib/contexts/game.svelte.ts`)

**Purpose**: Global game state management (variables, progress tracking)

**Features**:
- Per-toon state management
- localStorage persistence
- Variable get/set operations
- Progress tracking (chapters, labels, etc.)
- Timestamps for created_at/updated_at

**API**:
```typescript
type GameContext = {
  state: {
    toonId: string;
    variables: Record<string, string | number>;
    progress: Record<string, any>;
    lastCheckpoint: string | null;
    created_at: string;
    updated_at: string;
  };
  getVariable: (key: string) => string | number | undefined;
  setVariable: (key: string, value: string | number) => void;
  addVariable: (key: string, amount: number) => void;
  saveState: () => void;
  loadState: () => void;
}
```

**Usage Pattern**:
```typescript
// In layout or toon root
const gameContext = setGameContext(toonId);

// In components
const game = getGameContext();
game.setVariable('cucumbers', 5);
```

---

### 1.2 VisualNovelContext (`lib/contexts/visual-novel.svelte.ts`)

**Purpose**: VN-specific state (characters, active choices, scene state, UI visibility)

**Features**:
- Character data loading (per language)
- Active scene character management
- Pending choices tracking
- Choice cleanup on selection
- **UI state management** (message box, choices visibility)

**API**:
```typescript
export interface MessageBoxData {
  character: Character | null;
  text: string;
  wait: boolean;
}

type VisualNovelContext = {
  state: {
    // Data
    characters: Map<string, Character>;
    sceneCharacters: VNCharacterRef[];
    pendingChoices: Choice[];
    isProcessingEvents: boolean;
    
    // UI State (for components to consume)
    messageVisible: boolean;
    message: MessageBoxData;
    choicesVisible: boolean;
  };
  
  // Data methods
  loadCharacters: (lang: string) => Promise<void>;
  getCharacter: (name: string) => Character | undefined;
  joinScene: (characters: VNCharacterRef[]) => void;
  leaveScene: (characters: VNCharacterRef[]) => void;
  
  // Choice methods
  addChoice: (choice: Choice) => void;
  clearChoices: () => void;
  
  // UI methods (called by EventProcessor)
  showMessage: (character: Character | null, text: string, wait: boolean) => void;
  hideMessage: () => void;
  showChoices: () => void;
  hideChoices: () => void;
}

type Choice = {
  label: string;
  onSelect?: VNVariableAction;
};
```

**Pattern**: EventProcessor calls context methods → Context updates state → Components react

---

## Phase 2: Event Processing

### 2.1 EventProcessor Service (`lib/services/event-processor.ts`)

**Purpose**: Sequential VN event execution engine

**Key Methods**:
```typescript
class EventProcessor {
  constructor(
    events: VNEvent[],
    gameContext: GameContext,
    vnContext: VisualNovelContext
  );
  
  async run(): Promise<string | null>; // Returns jumpTo label or null
  async processEvent(event: VNEvent): Promise<{ jumpTo?: string }>;
  cleanup(): void;
}
```

**Event Handlers** (one per event type):
- `handleShowMessage()` - Display dialogue, wait for click if needed
- `handleShowChoices()` - Activate choice container
- `handleAddChoice()` - Add choice button, wire up click handler
- `handleJoinScene()` - Add characters to scene
- `handleLeaveScene()` - Remove characters from scene
- `handleEnd()` - Return jumpTo label

**Features**:
- Sequential async processing
- Early exit on `end` event
- Promise-based blocking/non-blocking events
- Cleanup of event handlers

---

## Phase 3: Visual Novel Utilities

### 3.1 Variable Actions (`lib/visual-novel/variable-actions.ts`)

**Purpose**: Reusable handlers for variable manipulation (NOT Svelte actions)

**Note**: These are VN-specific utilities, not Svelte's `use:action` directives.

```typescript
type VariableAction = VNVariableSetter | VNVariableAdder;

export function executeVariableAction(
  action: VariableAction,
  gameContext: GameContext
): void {
  if ('set' in action) {
    gameContext.setVariable(action.set.variable, action.set.value);
  } else if ('add' in action) {
    gameContext.addVariable(action.add.variable, action.add.value);
  }
}
```

**Usage**: Called when choice is selected

**Directory Structure**:
```
lib/visual-novel/
├── variable-actions.ts
├── event-handlers.ts (future)
└── index.ts (exports)
```

---

## Phase 4: UI Components

### 4.1 MessageBox Component (`lib/components/toon/vn/message-box.svelte`)

**Props**: None - reads directly from VN context

**Features**:
- Character name display
- Text rendering
- Continue hint indicator (if waiting)
- Click handler for progression
- **Handles own visibility** via internal state and CSS classes
- **Self-contained** - reads state from VN context directly

**State Management**:
- Accesses VN context internally via `getVisualNovelContext()`
- Uses `visible` class to show/hide (transition-ready)
- Always rendered in DOM, visibility toggled via CSS
- No props needed - cleaner parent component

**Example**:
```svelte
<script lang="ts">
  import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
  
  const vn = getVisualNovelContext();
  
  // Reactive state from context
  const visible = $derived(vn.state.messageVisible);
  const character = $derived(vn.state.message.character);
  const text = $derived(vn.state.message.text);
  const wait = $derived(vn.state.message.wait);
</script>

<div class="vn-message-box" class:visible>
  {#if character}
    <div class="character-name">{character.name}</div>
  {/if}
  {#if text}
    <div class="text">{text}</div>
  {/if}
  {#if wait}
    <div class="continue-hint">▼</div>
  {/if}
</div>

<style>
  .vn-message-box {
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.3s;
  }
  
  .vn-message-box.visible {
    opacity: 1;
    pointer-events: auto;
  }
</style>
```

---

### 4.2 Choices Component (`lib/components/toon/vn/choices.svelte`)

**Props**:
- `choices?: Choice[]` - Array of choice data
- `visible?: boolean` - Visibility state
- `onSelect?: (choice: Choice) => void` - Selection callback

**Features**:
- Renders list of choice buttons
- Handles selection
- Cleanup on selection
- Fade-in animation
- **Handles own visibility** via internal state and CSS classes

**State Management**:
- Always rendered in DOM, visibility toggled via CSS
- Shows/hides based on `visible` prop and whether choices array has items
- No conditional rendering needed in parent

**Example**:
```svelte
<script lang="ts">
  import type { Choice } from '$lib/visual-novel';
  
  let { 
    choices = [], 
    visible = false,
    onSelect 
  }: { 
    choices?: Choice[]; 
    visible?: boolean;
    onSelect?: (choice: Choice) => void;
  } = $props();
  
  const isVisible = $derived(visible && choices.length > 0);
  
  function handleSelect(choice: Choice) {
    onSelect?.(choice);
  }
</script>

<div class="vn-choices" class:visible={isVisible}>
  {#each choices as choice}
    <button class="vn-choice" onclick={() => handleSelect(choice)}>
      {choice.label}
    </button>
  {/each}
</div>

<style>
  .vn-choices {
    opacity: 0;
    pointer-events: none;
    transform: translateY(20px);
    transition: all 0.3s ease;
  }
  
  .vn-choices.visible {
    opacity: 1;
    pointer-events: auto;
    transform: translateY(0);
  }
</style>
```

---

### 4.3 Characters Component (`lib/components/toon/vn/characters.svelte`)

**Props**:
- `characters: VNCharacterRef[]` - Active scene characters

**Features**:
- Front/back positioning (relative to message box)
- Avatar vs full-body variants
- Mood/expression changes
- Stacking/ordering support

**Layout Slots**:
- Back Left
- Back Right
- Front Left (avatar)
- Front Right (avatar)

---

### 4.4 Main Visual Novel Component (`lib/components/toon/interactions/visual-novel.svelte`)

**Purpose**: VN UI overlay (does NOT render background - that's handled by renderer)

**Responsibilities**:
- Initialize EventProcessor with group events
- Display VN UI elements (message box, choices, characters)
- Pass visibility state to child components (they handle their own show/hide)
- Call `onComplete` callback with jumpTo label when events finish

**Structure**:
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import type { Group } from '$lib/models/content';
  import { getGameContext } from '$lib/contexts/game.svelte';
  import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
  import EventProcessor from '$lib/services/event-processor';
  import MessageBox from '../vn/message-box.svelte';
  import Choices from '../vn/choices.svelte';
  import Characters from '../vn/characters.svelte';
  
  let { group, onComplete }: { 
    group: Group, 
    onComplete: (jumpTo: string | null) => void 
  } = $props();
  
  const game = getGameContext();
  const vn = getVisualNovelContext();
  
  let processor: EventProcessor;
  
  onMount(async () => {
    await vn.loadCharacters(/* lang */);
    processor = new EventProcessor(group.events, game, vn);
    
    // EventProcessor updates vn.state as events run
    const jumpTo = await processor.run();
    onComplete(jumpTo);
  });
</script>

<div class="vn-overlay">
  <!-- Characters layer -->
  <Characters characters={vn.state.sceneCharacters} />
  
  <!-- UI Layer - always rendered, components handle own visibility -->
  <!-- Components read directly from context - no props needed! -->
  <MessageBox />
  <Choices />
</div>

<style>
  .vn-overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  
  .vn-overlay > :global(*) {
    pointer-events: auto;
  }
</style>
```

**Key Points**:
- Components are **always rendered** in DOM
- Visibility is controlled via props from VN context state
- Components handle their own show/hide transitions via CSS
- No `{#if}` conditionals needed - cleaner parent component
- **Reactive data flow**: EventProcessor → VN Context → Components

**EventProcessor Integration**:
EventProcessor receives VN context and calls methods like:
```typescript
// In event handlers
vn.showMessage(character, text, wait);  // Updates vn.state.message & messageVisible
vn.showChoices();                        // Updates vn.state.choicesVisible
vn.addChoice(choiceData);                // Adds to vn.state.pendingChoices
```

Components automatically react to state changes via `$derived`.

---

## Phase 5: Integration with Group System

### 5.1 Update `group.svelte`

**Architecture**: Load BOTH renderer AND interaction. Renderer shows background, interaction overlays UI.

**Current**:
```svelte
{#if group.renderer?.type === Renderer.VERTICAL}
  <Vertical group={group} />
{:else if group.renderer?.type === Renderer.HORIZONTAL}
  <Horizontal group={group} />
{/if}
```

**New** (with interaction support):
```svelte
<script lang="ts">
  import { Interaction, Renderer } from '$lib/models/enums';
  import Vertical from './renderer/vertical.svelte';
  import Horizontal from './renderer/horizontal.svelte';
  import VisualNovel from './interactions/visual-novel.svelte';
  import Webtoon from './interactions/webtoon.svelte';
  
  let { group }: { group: Group } = $props();
  let jumpToLabel = $state<string | null>(null);
  
  function handleInteractionComplete(label: string | null) {
    jumpToLabel = label;
    // Emit event or trigger navigation to next group
  }
</script>

<div class="group-container">
  <!-- Renderer Layer (background/slices) -->
  {#if group.renderer?.type === Renderer.VERTICAL}
    <Vertical {group} />
  {:else if group.renderer?.type === Renderer.HORIZONTAL}
    <Horizontal {group} />
  {/if}

  <!-- Interaction Layer (overlays on top) -->
  {#if group.interaction === Interaction.VISUAL_NOVEL}
    <VisualNovel {group} onComplete={handleInteractionComplete} />
  {:else if group.interaction === Interaction.WEBTOON}
    <Webtoon {group} onComplete={handleInteractionComplete} />
  {/if}
</div>

<style>
  .group-container {
    position: relative;
  }
</style>
```

**Key Changes**:
- Both renderer and interaction can be present simultaneously
- Renderer handles background rendering
- Interaction overlays VN UI elements on top
- Container uses relative positioning for overlay stacking

---

### 5.2 Group Navigation Handler

**Challenge**: Handle `jumpTo` labels between groups

**Solution**: Add group map at chapter level

```typescript
// In +page.svelte or +page.ts
const groupMap = new Map(
  chapter.groups
    .map((g, i) => g.label ? [g.label, i] : null)
    .filter(Boolean)
);

function jumpToGroup(label: string) {
  const index = groupMap.get(label);
  if (index !== undefined) {
    currentGroupIndex = index;
  }
}
```

---

## Phase 6: Testing & Polish

### 6.1 Test Data Structure

**Sample VN Group** (`test-vn-group.json`):
```json
{
  "label": "intro_scene",
  "interaction": "visual-novel",
  "renderer": {
    "type": "vertical"
  },
  "slices": [
    {
      "image": "backgrounds/bedroom.jpg"
    }
  ],
  "events": [
    {
      "type": "joinScene",
      "params": {
        "characters": [
          {
            "name": "Alex",
            "position": {
              "variant": "avatar",
              "relativeToMessageBox": "front",
              "order": 1
            }
          }
        ]
      }
    },
    {
      "type": "showMessage",
      "params": {
        "text": "Good morning!",
        "character": "Alex",
        "wait": true
      }
    },
    {
      "type": "showChoices",
      "params": {}
    },
    {
      "type": "addChoice",
      "params": {
        "label": "Say hello",
        "onSelect": {
          "set": {
            "variable": "alex_greeting",
            "value": "friendly"
          }
        }
      }
    },
    {
      "type": "addChoice",
      "params": {
        "label": "Ignore",
        "onSelect": {
          "set": {
            "variable": "alex_greeting",
            "value": "cold"
          }
        }
      }
    },
    {
      "type": "end",
      "params": {
        "jumpTo": "next_scene"
      }
    }
  ]
}
```

### 6.2 CSS Styling

**Files to create**:
- `lib/styles/visual-novel.css` - Core VN styles
- Component-specific styles in `.svelte` files

**Key Classes**:
- `.vn-container` - Full viewport container
- `.vn-background` - Background image layer
- `.vn-characters` - Character positioning layer
- `.vn-ui` - UI overlay (message box, choices)
- `.vn-message-box` - Dialogue container
- `.vn-choices` - Choice button container
- `.vn-character` - Individual character sprite

---

## Implementation Checklist

### Phase 1: Contexts (Svelte glue layer)
- [ ] `lib/contexts/game.svelte.ts`
- [ ] `lib/contexts/visual-novel.svelte.ts`
- [ ] Initialize contexts in appropriate layout/route

### Phase 2: Services
- [ ] `lib/services/event-processor.ts`
- [ ] Event handler functions

### Phase 3: VN Utilities (NOT Svelte actions)
- [ ] `lib/visual-novel/variable-actions.ts`
- [ ] `lib/visual-novel/index.ts` (barrel exports)

### Phase 4: Components
- [ ] `lib/components/toon/vn/message-box.svelte`
- [ ] `lib/components/toon/vn/choices.svelte`
- [ ] `lib/components/toon/vn/characters.svelte`
- [ ] `lib/components/toon/interactions/visual-novel.svelte` (overlay only)
- [ ] `lib/components/toon/interactions/webtoon.svelte` (already created)

### Phase 5: Integration
- [ ] Update `lib/components/toon/group.svelte` (load BOTH renderer + interaction)
- [ ] Add group navigation logic in chapter page
- [ ] Handle `jumpTo` labels between groups

### Phase 6: Testing
- [ ] Create test VN group data
- [ ] Test event flow
- [ ] Test variable persistence
- [ ] Test character loading
- [ ] Test choice handling
- [ ] Test scene transitions
- [ ] Test renderer + interaction overlay stacking

---

## Key Differences from Old App

1. **Reactive State**: Use Svelte runes instead of class properties
2. **Component-Based**: UI as components, not HTML string templates
3. **Context API**: Replace singleton instances with Svelte contexts
4. **Type Safety**: Full TypeScript support throughout
5. **Modern Patterns**: Async/await, proper cleanup, scoped styles
6. **Separation of Rendering**: Renderer components handle backgrounds, interaction components overlay UI

---

## Architecture Summary (Revised)

### Clear Separation of Concerns

**Contexts** (`lib/contexts/`)
- Svelte-specific glue layer using `setContext/getContext`
- `game.svelte.ts` - Game state management
- `visual-novel.svelte.ts` - VN-specific state

**VN Utilities** (`lib/visual-novel/`)
- Pure TypeScript modules (NOT Svelte actions like `use:action`)
- `variable-actions.ts` - Variable manipulation helpers
- Framework-agnostic business logic

**Services** (`lib/services/`)
- `event-processor.ts` - Event queue execution engine
- Not context-bound, instantiated as needed

**Components**
- **Renderers** (`lib/components/toon/renderer/`) - Display slices/background
- **Interactions** (`lib/components/toon/interactions/`) - UI overlay logic
- **VN UI** (`lib/components/toon/vn/`) - Reusable VN elements

### Data Flow

```
Chapter Page
  └─> Group Component
       ├─> Renderer (vertical/horizontal) - Shows slices
       └─> Interaction (visual-novel/webtoon) - Overlays UI
            └─> EventProcessor - Runs events
                 ├─> Uses GameContext (variables)
                 ├─> Uses VNContext (characters/choices)
                 └─> Calls VN utilities (variable-actions)
```

---

## Notes

- **Character Loading**: Needs API endpoint for character data (similar to old app)
- **Storage Keys**: Use `scutoons_game_${toonId}` for localStorage
- **Cleanup**: Ensure proper cleanup of event listeners on component unmount
- **Accessibility**: Add ARIA labels to interactive elements
- **Mobile**: Ensure touch-friendly tap targets for choices/messages
- **Performance**: Consider virtual scrolling for long choice lists (future)

---

## API Requirements

### Character Data Endpoint
```
GET /api/{toonId}/{lang}/characters.json
```

**Response**:
```json
[
  {
    "name": "Alex",
    "image": "characters/alex/full.png",
    "avatar": "characters/alex/avatar.png",
    "description": "Protagonist",
    "moods": {
      "happy": "characters/alex/happy.png",
      "sad": "characters/alex/sad.png",
      "angry": "characters/alex/angry.png"
    }
  }
]
```

---

## Future Enhancements

- **Conditional Events**: Events with `if` conditions based on variables
- **Animations**: Character enter/exit transitions
- **Sound Effects**: Per-message or per-character voice samples
- **Text Effects**: Typewriter effect, text speed control
- **Save/Load**: Manual save points, auto-saves at checkpoints
- **History**: Backlog of previous messages
- **Skip/Auto**: Auto-advance and skip read text modes
- **Gallery**: Unlock CG viewer based on progress
