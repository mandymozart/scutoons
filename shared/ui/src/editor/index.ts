// @scutoons/ui/editor: the editor kit of the web tools (RULES H9) – floating panels with universal tabs and the panels'
// controls, on the editor tokens (@scutoons/tokens editor.css, `--editor-*`). Night by default, `data-theme="day"` the variant.
export { default as FloatingPanels } from './floating-panels.svelte';
export { default as FloatingPanel } from './floating-panel.svelte';
export { Floats, PANELS_KEY, type HeldTab } from './floats.svelte.ts';
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
