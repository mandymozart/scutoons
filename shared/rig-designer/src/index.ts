// @scutoons/rig-designer: the rig designer's tabs (agents/research/RIG-EDITING.md §2, §8) for /live/deck next to the
// running engine – the Rust tab now (files, CodeMirror 6 loaded lazily, Compile, problems, console). The app calls
// setupRigDesigner({ locale }) once; the creator can mount the same components later. No app state, routing or paraglide.
export { setupRigDesigner, designerLocale, type RigDesignerSetup } from './setup.ts';
export { m as designerMessages, designerMessage, DESIGNER_MESSAGES, type DesignerMessageKey } from './messages.ts';
export { RigDesigner } from './designer.svelte.ts';
export { createDesignerApi, failureOf, type DesignerApi, type DesignerApiOptions } from './api.ts';
export * from './types.ts';
export * from './files.ts';
export * from './diagnostics.ts';
export { ConsoleBuffer, type ConsoleLine, type ConsoleLevel, type ConsoleOptions } from './console.ts';
export { summarize, headline, detailLines, type DescriptorSummary } from './descriptor.ts';
export { failureText, problemText, nameProblemText, isQuiet } from './reasons.ts';
export { STARTER_FILES } from './starter.ts';
export { default as RustTab } from './components/rust-tab.svelte';
export { default as DesignerTabs, type DesignerTabId } from './components/designer-tabs.svelte';
