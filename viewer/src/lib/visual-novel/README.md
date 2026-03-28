# Visual Novel System

## Architecture

The VN event processing logic is now **inline in the Svelte component** (`interactions/visual-novel.svelte`) to have direct access to SvelteKit's `goto()` function.

### Files:

**Core:**
- `interactions/visual-novel.svelte` - Event processing & navigation (main logic)
- `contexts/visual-novel.svelte.ts` - VN state management

**Utilities:**
- `utils.ts` - `interpolateVariables()` helper
- `jump-to.ts` - `buildJumpToUrl()` helper  
- `variable-actions.ts` - `executeVariableAction()` helper

**Deprecated:**
- ~~`event-processor.ts`~~ - Logic moved to `visual-novel.svelte`

## Navigation Flow

All navigation now uses `goto()` directly from Svelte components:

1. **Group Events** → `visual-novel.svelte` → `goto()`
2. **Choice Selection** → `choices.svelte` → `goto()`
3. **Slice Triggers** → `triggers/*.svelte` → `goto()`

No more callback chains or pending state!
