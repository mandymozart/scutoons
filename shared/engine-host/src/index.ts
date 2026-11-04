// @scutoons/engine-host (#298): the deck's browser engine as a component – `createEngineHost` (the controller) and
// `<EngineHost>` (its being's canvas). research/EDITOR.md: the creator's Sound design workspace and, later, the scene
// editor run a deck instance through it; the viewer's /live/deck brings its engine up through it too.
export { EngineHost, type EngineHooks } from './controller.ts';
export { createEngineHost } from './host.ts';
export { engineFiles } from './files.ts';
export { FALL_SECONDS, MAX_OUTPUTS, OutputMeters } from './meters.ts';
export { ACTIVITY_SECONDS, SIGNAL_KINDS, SignalMonitor, describeSignal, emptySignals, type SignalKind, type SignalState, type SignalsView } from './monitor.ts';
export { EngineLog, formatLine, messageOf, type LogLevel, type LogLine } from './log.ts';
export { deviceInfo, engineReport, playoutOf, type Playout, type ReportInput } from './report.ts';
export { deckOptions, folder, moodChoices, rigSubject, type EngineHostOptions } from './options.ts';
export { canStart, canStop, emptySnapshot, transportNote, type EnginePhase, type EngineSnapshot } from './status.ts';
export { GATE_EVENTS, StartGate, gateNext, gateOpen, listenForGesture, type GateEvent, type GateState } from './gate.ts';
export { default as EngineStage } from './engine-stage.svelte';
