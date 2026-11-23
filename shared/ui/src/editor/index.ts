// @scutoons/ui/editor: the editor kit of the web tools (RULES H9) – floating panels with universal tabs and the panels'
// controls, on the editor tokens (@scutoons/tokens editor.css, `--editor-*`). Night by default, `data-theme="day"` the variant.
export { default as FloatingPanels } from './floating-panels.svelte';
export { default as FloatingPanel } from './floating-panel.svelte';
export { Floats, PANELS_KEY, type FloatsLayout, type HeldTab, type StageZone, type Thrown } from './floats.svelte.ts';
// taking the stage: a view on the stage in one of four layouts, its drop zones at the screen's edges
export { default as StageZones } from './stage-zones.svelte';
export { VIEW_STATES, slotRects, viewsOn, type Cell, type StageLayout, type StageState, type ViewState } from './stage.ts';
export { at, type Corner, type Dimension, type Metrics, type PanelSpec, type Place, type Rect, type Size, type View } from './floats.ts';
export { type Arrangement, type DropTarget, type TabSpec } from './panel-tabs.ts';
export { default as Section } from './section.svelte';
export { sections } from './sections.svelte.ts';
export { default as Toggle } from './toggle.svelte';
export { default as ToolGroup, type ToolGroupOption } from './tool-group.svelte';
export { default as ToolMenu } from './tool-menu.svelte';
export { type ToolOption } from './tool-groups.ts';
// the one number field of the tools (typed maths, units, reset) – the same component for the editor kit
export { default as ScrubNumber } from '../scrub-number.svelte';
export { measureDrop } from './measure-drop.ts';
// the UI kit (RULES H12): deck-kit's editor controls as components – sizes, radii, insets from the tokens (tools.css
// --kit-*); a Pill rounds what sits in it concentric with itself
export { default as Button } from './button.svelte';
export { default as IconButton } from './icon-button.svelte';
export { default as Pill } from './pill.svelte';
export { default as PillTab } from './pill-tab.svelte';
export { default as PillTabs, type PillTabItem } from './pill-tabs.svelte';
export { default as Menu } from './menu.svelte';
export { default as MenuPanel } from './menu-panel.svelte';
export { default as MenuItem } from './menu-item.svelte';
export { default as MenuHead } from './menu-head.svelte';
export { default as Field } from './field.svelte';
export { default as ListRow } from './list-row.svelte';
export { default as Tile } from './tile.svelte';
export { default as Card } from './card.svelte';
export { default as Chip } from './chip.svelte';
export { default as Select, type SelectOption } from './select.svelte';
// the overlay level (RULES H15): the switcher, Search, a panel taken full, the dialogs – over everything, blurred under
export { default as Overlay } from './overlay.svelte';
export { onOverlayLevel, useOverlayLevel } from './overlay-level.ts';
// gestures and motion: a fling closes, a swipe up from the bottom; the motion tokens read in ms for script-timed motion
export { FLING, addSample, atBottomEdge, flingEdge, isSwipeUp, throwOffset, velocity, type FlingEdge, type Sample } from './gestures.ts';
export { motionMs, type MotionSpeed } from './motion.ts';
