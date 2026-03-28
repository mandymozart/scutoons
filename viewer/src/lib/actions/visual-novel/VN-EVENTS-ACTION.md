# VN Events Action

Svelte action for triggering Visual Novel events on slices with click, intersection, or hover modes.

## Architecture

```
┌─────────────────────────────────────┐
│  getCurrentVNContext()             │  ← Module-level singleton
│  getCurrentGameContext()           │  ← No manual injection needed!
└─────────────────────────────────────┘
                 ▲
                 │
    ┌────────────┼────────────┐
    │            │            │
┌───┴───┐   ┌───┴───┐   ┌───┴────┐
│ Click │   │Intersec│  │ Hover  │  ← Event handlers
│Handler│   │ -tion  │  │Handler │
└───┬───┘   └───┬───┘   └───┬────┘
    │           │           │
    └───────────┼───────────┘
                ▼
        EventProcessor
                │
                ▼
        Returns jumpTo URL
                │
                ▼
    onNavigate callback → goto()
```

## Usage

### 1. JSON Configuration

Add `events` array to slice with `trigger` property:

```json
{
  "image": "slice-001.jpg",
  "events": [
    {
      "type": "showMessage",
      "trigger": { "mode": "click", "once": true },
      "params": {
        "text": "You clicked the slice!",
        "wait": true
      }
    },
    {
      "type": "jumpTo",
      "trigger": { "mode": "intersection" },
      "params": {
        "target": {
          "type": "chapter",
          "value": "2",
          "label": "scene-200"
        }
      }
    }
  ]
}
```

### 2. Component Usage

The action is already integrated in `slice.svelte`:

```svelte
<div 
  use:vnEvents={{ 
    events: slice.events, 
    onNavigate: handleNavigate 
  }}
>
  <img src={imageUrl} />
</div>
```

## Event Types

All standard VN events supported:
- `showMessage` - Display message
- `showChoices` - Show choice buttons
- `addChoice` - Add choice option
- `joinScene` / `leaveScene` - Manage characters
- `hideMessage` - Hide message
- `showOnomatopoeia` - Show sound effects
- `jumpTo` - Navigate to another chapter/scene

## Trigger Modes

### `click`
- Triggers when user clicks the slice
- Default: `once: true` (only triggers first click)

### `intersection`
- Triggers when slice enters viewport
- Uses IntersectionObserver with `threshold: 0.5`
- Default: `once: true` (only triggers first intersection)

### `hover` (planned)
- Triggers on mouse enter
- Not yet implemented

## Configuration

```typescript
interface VNEventsConfig {
  events?: VNEventWithTrigger[];  // Events with trigger metadata
  onNavigate?: (url: string) => void;  // Callback for jumpTo
}

type VNEventWithTrigger = VNEvent & {
  trigger?: {
    mode: 'click' | 'intersection' | 'hover';
    once?: boolean;  // Default: true
  };
};
```

## Implementation Details

### Context Access
- Uses `getCurrentVisualNovelContext()` and `getCurrentGameContext()`
- No manual context injection needed (singleton pattern from audio actions)
- Contexts must be initialized higher in component tree

### Navigation
- `jumpTo` events return URL string
- Component receives URL via `onNavigate` callback
- Component calls `goto()` from proper Svelte context
- Prevents cleanup issues with direct navigation

### Event Processing
- Reuses `EventProcessor` for consistency
- Strips `trigger` property before processing
- Independent from group event sequences
- Supports wait states and choice interactions

## Example Scenarios

### Click to Navigate
```json
{
  "type": "jumpTo",
  "trigger": { "mode": "click" },
  "params": {
    "target": { "type": "chapter", "value": "3" }
  }
}
```

### Auto-trigger on Scroll
```json
{
  "type": "showMessage",
  "trigger": { "mode": "intersection", "once": true },
  "params": {
    "text": "You've reached the secret area!",
    "wait": false
  }
}
```

### Multiple Events
```json
{
  "events": [
    {
      "type": "showMessage",
      "trigger": { "mode": "click" },
      "params": { "text": "First message" }
    },
    {
      "type": "showMessage",
      "trigger": { "mode": "click" },
      "params": { "text": "Second message" }
    }
  ]
}
```
Events trigger in sequence on each click.
