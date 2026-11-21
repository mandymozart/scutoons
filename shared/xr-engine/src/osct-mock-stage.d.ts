// What stage.ts uses of osct/editor/src/lib/stage/mock-stage.ts. That one file does not type-check against
// SCUTOONS's three.js (a texture's `image` is `unknown` in the types of 0.186, OSCT builds against 0.174), and the
// copy stays unchanged: tsconfig.json leaves it out of the check and maps its name to this declaration. The code that
// runs is OSCT's file.
import type { Scene } from 'three';
import type { StageView } from '#lib/stage/mock-tracker.js';

/** `assetUrl`: where the stage's own files live (`models/…`, `textures/…`, `hdri/…`) */
export declare const MockStage: new (scene: Scene, assetUrl?: (path: string) => string) => StageView;
