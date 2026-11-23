import type * as THREE from "three";
import type { Material, Object3D } from "three";
import { type AppearanceSpec, type Moment, registerAppearanceSpec, unregisterAppearanceSpec } from "../types/index";

/**
 * The looks of a plugin's own: how an entity appears and disappears, in code. The content names a look by its type
 * for a moment (`experience.appearance: { reveal, found, lost }`, `shared/types/appearance.ts`) – a built-in one, which the
 * engine's own animation plays from its values (`./celebration`), or one registered here through the plugin API's
 * `registerAppearance` with the plugin as the owner; it then plays in the built-in one's place.
 *
 *   api.registerAppearance("softFade", SPEC, ({ params, materials }) => {
 *     const surface = materials();
 *     return {
 *       duration: params.duration,
 *       update: visible => surface.forEach(material => { material.transparent = true; material.opacity = visible; }),
 *     };
 *   });
 */

/** What a look's factory gets, once per animation */
export interface AppearanceContext<P = Record<string, unknown>> {
  /**
   * The entity on its anchor – its whole node tree. Its position, rotation and scale are its placement; whatever the
   * look sets on them is put back when the animation ends
   */
  object: Object3D;
  /** The target's anchor: the page – x to the right, y to the top of the page, z out of it, 1 unit = the target's width */
  anchor: Object3D;
  /** The look's parameters with the spec's defaults filled in */
  params: P;
  /** What happens: the first find (`reveal`), a later one (`found`), the page is lost (`lost`) */
  moment: Moment;
  /** The app's three.js (one instance – a plugin never imports three itself) */
  three: typeof THREE;
  /**
   * Copies of the entity's materials for this animation, on its meshes until it ends (a model's own materials are
   * shared with every entity of the same file – never change those): set opacity, colour or a shader of your own on these
   */
  materials(): Material[];
}

/** One running animation of a look */
export interface AppearanceRun {
  /** ms */
  duration: number;
  /**
   * Show the entity `visible` of the way there: 0 = nothing of it, 1 = as it is. `reveal` and `found` run from 0 to 1,
   * `lost` from 1 to 0 – a look that only says how the entity looks at `visible` covers all three
   */
  update(visible: number): void;
  /** Free what the look created; the engine puts the placement and the materials back itself */
  dispose?(): void;
}

export type AppearanceFactory<P = Record<string, unknown>> = (context: AppearanceContext<P>) => AppearanceRun;

export interface AppearanceRegistration {
  owner: string;
  spec: AppearanceSpec;
  factory: AppearanceFactory;
}

const registry = new Map<string, AppearanceRegistration>();

/**
 * Register a look: its spec (parameters with kind, default and range, the same shape as a component's) and the factory
 * that plays it. A type registered by another owner is refused; the same owner replaces its own.
 */
export const registerAppearance = (type: string, spec: AppearanceSpec, factory: AppearanceFactory, owner: string): void => {
  const existing = registry.get(type);
  if (existing && existing.owner !== owner) throw new Error(`The look "${type}" is registered by ${existing.owner}`);
  registry.set(type, { owner, spec, factory });
  // The checks of an experience's and an entity's looks read the spec from the contract's registry
  registerAppearanceSpec(type, spec);
};

/** Every look an owner registered leaves the registry (a plugin is uninstalled) */
export const unregisterAppearancesOf = (owner: string): void => {
  registry.forEach((registration, type) => {
    if (registration.owner !== owner) return;
    registry.delete(type);
    unregisterAppearanceSpec(type);
  });
};

export const appearanceRegistration = (type: string): AppearanceRegistration | undefined => registry.get(type);

export const registeredAppearanceTypes = (): string[] => [...registry.keys()];
