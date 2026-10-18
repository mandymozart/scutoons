// @scutoons/pwa – what makes a SCUTOONS app an installable web app (agents/research/PWA.md, research/PWA-APPS.md): the
// install rules + state (every app), and for the tools (creator, admin, embassy) the app-shell service worker, its page
// side and the install / update bars. The reader keeps its own worker and bars (viewer/src/service-worker.ts).
export * from './install-rules.ts';
export * from './push-rules.ts';
export { Install, type InstallOptions } from './install.svelte.ts';
export { ToolPwa } from './tool-pwa.svelte.ts';
export { PushSwitch, type PushApi, type PushKind, type PushState, type PushSwitchOptions } from './push.svelte.ts';
export { startToolWorker, TOOL_STATIC, type ToolWorkerOptions, type WorkerScope } from './tool-worker.ts';
export { pwaLabels, type PwaLabels } from './labels.ts';
export { default as InstallBar } from './install-bar.svelte';
export { default as UpdateBar } from './update-bar.svelte';
export { default as NotificationsPage } from './notifications-page.svelte';
