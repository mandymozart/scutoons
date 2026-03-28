# Type Organization

The types have been organized into a cleaner structure that separates fundamental data models from application interfaces.

## Structure

```
app/
├── models/                    # Fundamental data models (JSON structure)
│   ├── enums.d.ts            # Core enumerations
│   ├── audio.d.ts            # Audio-related models
│   ├── visual-novel.d.ts     # VN events, characters
│   ├── content.d.ts          # Chapter, Season, Song, Group, etc.
│   └── membership.d.ts       # Membership data
│
├── stores/
│   └── types.d.ts            # IGameStore, IMembershipStore, INavigationStore
│
├── controllers/
│   ├── events/
│   │   └── types.d.ts        # IVisualNovelEvents
│   └── interactions/
│       └── types.d.ts        # IInteraction, IInteractionManager
│
├── views/
│   └── types.d.ts            # IChapterView
│
├── renderers/
│   └── types.d.ts            # IRenderer, RenderOptions
│
└── types.d.ts                # IScutoon + re-exports all types
```

## Philosophy

### Models (`models/`)
- **What they are:** Pure data structures that represent JSON content
- **Purpose:** Define the shape of data coming from APIs, files, or configuration
- **Examples:** `Chapter`, `VNEvent`, `AudioTrack`, `Membership`
- **No dependencies:** Models should not depend on application interfaces

### Application Interfaces
- **What they are:** Contracts for application components (stores, controllers, views)
- **Purpose:** Define the behavior and structure of runtime components
- **Location:** Next to their implementations
- **Examples:** `IGameStore` (in stores/), `IVisualNovelEvents` (in controllers/events/)

### Main Types File (`types.d.ts`)
- Contains the root application interface `IScutoon`
- Re-exports all types for convenience
- Single import point for backward compatibility

## Import Best Practices

### Recommended (specific imports)
```typescript
// Import from specific locations
import { Chapter, Group } from '../models/content';
import { IGameStore } from '../stores/types';
import { IVisualNovelEvents } from '../controllers/events/types';
```

### Also Valid (convenience imports)
```typescript
// Import from main types file
import { Chapter, IGameStore, IVisualNovelEvents } from '../types';
```

## Benefits

1. **Better organization:** Related types are grouped together
2. **Clearer dependencies:** You can see what depends on what
3. **Easier maintenance:** Find types next to their usage
4. **Type safety:** Models are separate from implementation details
5. **Scalability:** Easy to add new models or interfaces
