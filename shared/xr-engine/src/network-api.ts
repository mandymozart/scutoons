/**
 * What an app sees of the XR scope of the one network (#763 step 4, #482): the types of `./network`, with no import of
 * OSCT's files (as reader-api.ts for `./reader`). src/network.ts is the code and must fit these declarations.
 */
import type { NetworkView } from '@scutoons/api';

/**
 * One level of an XR experience's logic as the one network's view: OSCT's projection (editor/src/lib/logic/project.ts)
 * of the experience document (`experience:<id>`: its configuration and plugins), translated to the contract.
 * `context`: the level's address – `experience:<id>` (the whole experience), `…/group:<g>`, `…/scene:<s>`,
 * `…/scene:<s>/target:<t>` or `…/scene:<s>/target:<t>/node:<path>` (path `1.0`); an unknown one falls back to the experience.
 */
export declare function projectXr(document: unknown, context: string): NetworkView.View;
