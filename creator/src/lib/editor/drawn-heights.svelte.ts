// The slice canvas' drawn heights (% of the slice) of text boxes / placed items without `rect.height` (an auto height, an
// icon's own ratio), by the entry's path (`groups.0.slices.3.dialog.1`). The canvas measures them; the anchor picker
// needs them to keep a box in place when its anchor changes (anchor.ts `reanchor`).
import type { Path } from '@scutoons/content';

export const drawnHeights: Record<string, number> = $state({});

export const drawnKey = (entryPath: Path) => entryPath.join('.');
