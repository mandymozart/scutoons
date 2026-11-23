import * as THREE from "three";
import { Material, type Mesh, type Object3D, type Texture } from "three";
import { type EntityData, type MaterialSpec, resolveSpecParams, type Target } from "../types/index";
import { resolveTextures, sameTextures } from "./textures";
import { collectFeedbacks, type OwnedFeedbacks } from "./feedback";

/**
 * A node's material of a plugin (`api.registerMaterial`): the node's
 * `material: { type, params }` names a material a plugin registered; its `build` makes a three.js material (a
 * `ShaderMaterial` with GLSL of its own, a `MeshStandardMaterial` with a procedural texture …) and the engine puts it on
 * every mesh of the node's own object, in place of the material its type made (a model's, an image's or video's plane,
 * a plugin node's). The node's material layers then add on top (`utils/layers.ts` patches the plugin's material in
 * place – a `ShaderMaterial` takes them before `#include <colorspace_fragment>` and reads `vUv`). Texture parameters
 * arrive as three.js textures (`textures.ts`); `time` is a uniform the engine advances every frame, so a shader can use
 * it directly (`uniforms: { time }`).
 */

/** What a material's `build` gets */
export interface MaterialContext<P = Record<string, unknown>> {
  node: EntityData;
  /** The material's parameters with the spec's defaults filled in – a `texture` parameter as a three.js `Texture` (or null) */
  params: P;
  /** Seconds since the material was built, as a uniform the engine advances every frame (`uniforms: { time }`) */
  time: { value: number };
  /**
   * The picture of the material it replaces – an image's or video's texture, a model's base colour map – or null: a
   * material that keeps the node's own picture reads it here
   */
  map: Texture | null;
  target: Target;
  /** The app's three.js (one instance – plugins never import three themselves) */
  three: typeof THREE;
}

/** What `build` returns when it needs more than the material: hooks, as a node type's */
export interface MaterialInstance {
  material: Material;
  /** Every frame (seconds since the last one) – `time` advances by itself */
  update?(delta: number): void;
  /** Other parameter values in place (the editor's live edit); false = the material is built anew */
  tune?(params: Record<string, unknown>): boolean;
  /** Free what the build made beside the material (the engine disposes the material) */
  dispose?(): void;
}

/** Builds the material: a three.js material, or a `MaterialInstance` with hooks */
export type MaterialBuild<P = Record<string, unknown>> = (context: MaterialContext<P>) => Material | MaterialInstance | null | void;

export interface MaterialRegistration { plugin: string; spec: MaterialSpec; build: MaterialBuild }

const registrations = new Map<string, MaterialRegistration>();

/** The plugin API registers and unregisters materials here (`plugins.ts`) */
export const setMaterialRegistration = (type: string, registration: MaterialRegistration): void => {
  registrations.set(type, registration);
};
export const deleteMaterialRegistration = (type: string): void => {
  registrations.delete(type);
};
export const materialRegistration = (type: string): MaterialRegistration | undefined => registrations.get(type);

/**
 * A node type's mesh with `userData.keepMaterial = true` keeps its own material when the node carries a material: an
 * aquarium's glass and fish stay as they are, its water takes the node's material
 */
export const KEEP_MATERIAL = "keepMaterial";

/** Marks a material the node owns (a plugin's): layers patch it in place instead of a clone */
export const OWNED_MATERIAL = "osctOwned";

const meshesOf = (object: Object3D): Mesh[] => {
  const meshes: Mesh[] = [];
  object.traverse(child => { if ((child as Mesh).isMesh && child.userData[KEEP_MATERIAL] !== true) meshes.push(child as Mesh); });
  return meshes;
};

const mapOf = (meshes: Mesh[]): Texture | null => {
  for (const mesh of meshes) {
    for (const material of Array.isArray(mesh.material) ? mesh.material : [mesh.material]) {
      const map = (material as Material & { map?: Texture | null }).map;
      if (map) return map;
    }
  }
  return null;
};

/** The node's plugin material on its object: hooks for the node; `tune` false = build the node anew */
export interface AppliedMaterial {
  update(delta: number): void;
  dispose(): void;
  tune(next: EntityData): boolean;
}

/**
 * Put the node's material (`node.material`) on the meshes of `object`. Null when the node names none, nobody registered
 * its type (logged – the node keeps its own material), the object has no mesh, or `build` fails (logged).
 */
export const applyMaterial = (node: EntityData, object: Object3D, target: Target): AppliedMaterial | null => {
  const data = node.material;
  if (!data) return null;
  const registration = registrations.get(data.type);
  if (!registration) {
    console.warn(`[materials] No material "${data.type}" registered (target ${target.id}) – the node keeps its own`);
    return null;
  }
  const meshes = meshesOf(object);
  if (!meshes.length) {
    console.warn(`[materials] ${data.type} on a ${node.type} node without meshes (target ${target.id}) – ignored`);
    return null;
  }
  const raw = resolveSpecParams(registration.spec.params, data.params);
  const textures = resolveTextures(registration.spec.params, raw, `material ${data.type} on ${target.id}`);
  const time = { value: 0 };
  let built: Material | MaterialInstance | null | void;
  // The feedback buffers the build makes pause with the node's object and go with the material
  let feedbacks: OwnedFeedbacks | null = null;
  try {
    [built, feedbacks] = collectFeedbacks(() => registration.build({ node, params: textures.values, time, map: mapOf(meshes), target, three: THREE }));
    feedbacks.own(object);
  } catch (error) {
    console.error(`[materials] ${data.type} (plugin ${registration.plugin}) failed on ${target.id}:`, error);
    textures.dispose();
    return null;
  }
  const instance: MaterialInstance | null = built instanceof Material ? { material: built } : built && built.material instanceof Material ? built : null;
  if (!instance) {
    console.error(`[materials] ${data.type} (plugin ${registration.plugin}) returned no three.js material on ${target.id}`);
    feedbacks?.dispose();
    textures.dispose();
    return null;
  }
  const material = instance.material;
  material.userData[OWNED_MATERIAL] = true;
  const originals = meshes.map(mesh => mesh.material);
  meshes.forEach(mesh => (mesh.material = material));
  let applied = raw;
  return {
    update: delta => {
      time.value += delta;
      textures.update(delta);
      instance.update?.(delta);
    },
    dispose: () => {
      meshes.forEach((mesh, i) => (mesh.material = originals[i]));
      instance.dispose?.();
      feedbacks?.dispose();
      material.dispose();
      textures.dispose();
    },
    tune: next => {
      if (next.material?.type !== data.type) return false;
      const values = resolveSpecParams(registration.spec.params, next.material.params);
      if (!sameTextures(registration.spec.params, values, applied)) return false;
      const same = Object.keys(values).every(key => JSON.stringify(values[key]) === JSON.stringify(applied[key]));
      // The textures stay: hand the code the same texture objects with the other values
      const withTextures = { ...values, ...Object.fromEntries(Object.entries(textures.values).filter(([key]) => registration.spec.params[key]?.kind === "texture")) };
      const ok = same || (instance.tune?.(withTextures) ?? false);
      if (ok) applied = values;
      return ok;
    },
  };
};
