// @scutoons/ask – the Ask panel (CUTOVER 10.28): a chat with the SCUTOONS agent of the Ask service (tools/mcp), docked
// right in creator, embassy and admin. The app passes what differs: its name (the service picks the tools and the role
// by it), the chats API, the page context, labels, links for steps and an optional snippet for app-specific results.
export { default as AskPanel } from './ask-panel.svelte';
export { ask, askAbout, pagePanel } from './ask.svelte.ts';
export type { AskLabels } from './labels.ts';
