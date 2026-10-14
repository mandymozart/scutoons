// @scutoons/reader: the reader's rendering, shared by the viewer and the creator (identical output in both).
// Components take props and emit callbacks; app state (VN/game/audio contexts, routing, i18n) stays in the apps.
export { default as Slice } from './slice.svelte';
export { default as SliceVideo } from './slice-video.svelte';
export { default as Dialogs } from './dialog/dialogs.svelte';
export { default as Dialog } from './dialog/dialog.svelte';
export { default as SliceLayers } from './slice-layers.svelte';
export { default as SliceItem, type SliceItemInfo, type ResolveSliceItem } from './items/slice-item.svelte';
export { sliceAudio, type SliceAudioConfig, type SliceAudioPlayer } from './actions/slice-audio.ts';
export { sliceEvents, type SliceEventsConfig } from './actions/slice-events.ts';
export { getMusicStateForSlice } from './actions/music.ts';
export { isWithinRect } from './rect.ts';
export { default as CreditsView } from './credits/credits-view.svelte';
export type { ActiveTextBox, DialogPlay, PercentRect, SlicePlayerDetail, TextExpressions } from './types.ts';
export { SLICE_PLAYER_EVENT, SLICE_SKIP_EVENT } from './types.ts';
export { default as PlayerOverlay, DEFAULT_PLAYER_LABELS, type PlayerLabels } from './player-overlay.svelte';
export { default as CreditsColophon } from './credits/credits-colophon.svelte';
export { default as CoverArt } from './cover/cover-art.svelte';
export { MapViewport, mapGestures, type MapGestureOptions } from './map/viewport.svelte.ts';
export { default as MapRoutes, type DrawnRoute } from './map/map-routes.svelte';
export { default as MapTerrain } from './map/map-terrain.svelte';
export { configureFx, playFxSound, playRevealSound, buzz, isFxSound, FX_SOUNDS, type FxSound, type FxOutput } from './fx/sounds.ts';
export { fxLayer, liftCue, refuseCue, nudgeCue, sparkles, impactCue, IMPACTS, IMPACT_DEFAULT_MS, type Impact, type Lift } from './fx/cues.ts';
export { loadShape, type Shape } from './fx/shape.ts';
export { default as ScoreView, DEFAULT_SCORE_LABELS, type ScoreLabels } from './score/score-view.svelte';
