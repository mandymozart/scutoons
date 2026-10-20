// The storyboard renderer (#124, moved from tools/storyboard-render – STUDIO-IN-CREATOR §2.1): three.js, imported only
// by the storyboard page's 3D view (a dynamic import – the rest of the creator doesn't load three for it).
export { shotCamera, buildScene, disposeTree, type BuiltScene, type BuildOptions, type SceneLight } from './scene.ts';
export { sinPass, type SinPass, type LookParams } from './sin-pass.ts';
export { figureBoxes, drawDialog, drawMarks, loadFonts, seedOf, type FigureBox } from './texts.ts';
export { renderShot, type RenderOptions } from './render-shot.ts';
export { presetOf, presetStyle } from './text-presets.ts';
export type { ModelUrls } from './models.ts';
export type { Figurine } from './figures.ts';
