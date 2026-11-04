// @scutoons/live-deck (#197, #198): the browser deck – a phone joins the Cathedral of decks as a being. One module for
// the viewer's /live/deck and deck/lab's harness: the relay client (client.ts), the knobs' shape and the mood ladder (scene.ts – the scene is the core's, #360), the
// deck's state and sound (deck.ts), the being in 3D (space.ts – deck/space-web's wasm), the answer to the
// conductor's scene sync (sync.ts, #195), "save as preset" from the knobs (preset.ts, #270), the controls' bridge – sticks, selection, MIDI learn, the stats (controls.ts,
// midi.ts, #209).
export * from './client.ts';
export * from './scene.ts';
export * from './deck.ts';
export * from './space.ts';
export * from './rig.ts';
export * from './sync.ts';
export * from './controls.ts';
export * from './midi.ts';
export * from './gamepad.ts';
// #321: the core's tempo clock as the page reads it
export { beatAt, nowMs, SOURCES, type ClockSource, type ClockState } from '@scutoons/live-core';
