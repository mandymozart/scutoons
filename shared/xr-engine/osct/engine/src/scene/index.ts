// The AR scene and its building blocks – the only code that touches three.js. An app passes its tracker
// (`ArSceneOptions.createTracker`): the reader the 8th Wall engine (`./xr8-tracker`, kept out of this barrel so an
// app without it never loads it), the editor its mock tracker.
export * from "./ar-scene";
export * from "./entities";
export * from "./post/index";
export * from "./tracker-types";
export { onAppUi, UI_SELECTOR } from "./ui";
export * from "./plugin-runtime";
