# Models - Fundamental Data Types

This directory contains the fundamental data models that represent the structure of JSON data and core domain concepts.

## Files

### `enums.d.ts`
Core enumerations used throughout the application:
- `Renderer` - Rendering modes (vertical, horizontal)
- `Language` - Supported languages (de, en, jp, kr, es)
- `Status` - Publication status (published, draft, coming-soon)
- `Trigger` - Event triggers (scrollInView, hover, click)
- `Interaction` - Interaction types (webtoon, visual-novel)

### `audio.d.ts`
Audio-related data models:
- `AudioTrack`, `MusicEvent`, `Sample`
- `AudioConfiguration`, `AdoptiveMusicPlayerConfiguration`

### `visual-novel.d.ts`
Visual novel specific models:
- `Character`, `VNCharacterRef`
- `VNVariableSetter`, `VNVariableAdder`
- All VN event types: `VNShowMessageEvent`, `VNAddChoiceEvent`, etc.
- Union type: `VNEvent`

### `content.d.ts`
Content structure models:
- `Chapter`, `Season`, `Song`, `Group`, `Slice`
- `KeyVisual`, `Dialog`, `Onomatopoeia`
- `Cover`, `Credits`, `Toon`
- Reference types: `ChapterRef`, `GroupRef`

### `membership.d.ts`
User membership data model:
- `Membership`

## Usage

Import from specific model files:
```typescript
import { Chapter, Group } from '../models/content';
import { VNEvent } from '../models/visual-novel';
import { Renderer, Language } from '../models/enums';
```

Or import from the main types file (which re-exports everything):
```typescript
import { Chapter, VNEvent, Renderer } from '../types';
```
