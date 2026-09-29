// @scutoons/reader: the reader's rendering, shared by the viewer and the creator (identical output in both).
// Components take props and emit callbacks; app state (VN/game/audio contexts, routing, i18n) stays in the apps.
export { default as Slice } from './slice.svelte';
export { default as Dialogs } from './dialog/dialogs.svelte';
export { default as Dialog } from './dialog/dialog.svelte';
export { default as Onomatopoeia } from './onomatopoeia/onomatopoeia.svelte';
export { sliceAudio, type SliceAudioConfig, type SliceAudioPlayer } from './actions/slice-audio.ts';
export { sliceEvents, type SliceEventsConfig } from './actions/slice-events.ts';
export { getMusicStateForSlice } from './actions/music.ts';
export { isWithinRect } from './rect.ts';
export { default as CreditsView } from './credits/credits-view.svelte';
export type { ActiveOnomatopoeia, PercentRect } from './types.ts';
