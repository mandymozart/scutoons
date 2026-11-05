// @scutoons/rig-designer: the rig designer (agents/research/RIG-EDITING.md §2, §8) for the creator's Sound design workspace
// (#282) – the Rust tab (files, CodeMirror 6 loaded lazily, Save, Compile, problems, console), Parameters (read-only) and
// Nodes (read-only: the core's wiring of what the deck plays, placed and drawn, the live values on the cables).
// The app calls setupRigDesigner({ locale }) once and gives the designer a ModuleStore (the module's document). No app
// state, routing or paraglide.
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
export { MODULE_ID, isModuleId, moduleIdOf, moduleContent, readModuleContent, sameFiles, type CompileOutcome } from './store.ts';
export { default as RustTab } from './components/rust-tab.svelte';
export { default as ParametersTab } from './components/parameters-tab.svelte';
export { default as NodesTab } from './components/nodes-tab.svelte';
export { layoutGraph, cableText, cableTexts, NODE, type GraphIn, type GraphWiring, type GraphNode, type GraphLink, type GraphPort, type GraphLayout } from './nodes.ts';
export { default as DesignerTabs, type DesignerTabId } from './components/designer-tabs.svelte';
