import { Box3, BoxGeometry, type BufferGeometry, EdgesGeometry, LineBasicMaterial, LineSegments, type Material, Mesh, MeshBasicMaterial, MeshStandardMaterial, type Object3D, Points, Vector3 } from "three";
import type { ShadingMode } from "./preview";

const WIRE = new MeshBasicMaterial({ color: 0xd8d8d8, wireframe: true });
const SOLID = new MeshStandardMaterial({ color: 0xb4b4b4, roughness: 0.85, metalness: 0 });

const originals = new WeakMap<Mesh, Material | Material[]>();

/**
 * The mesh's own material, also while wireframe or solid shows a stand-in: a texture that arrives later (the stage's
 * sheet and print images) goes there, never into the shared stand-in
 */
export const ownMaterial = <M extends Material = Material>(mesh: Mesh): M => (originals.get(mesh) ?? mesh.material) as M;

/** Apply a mode to every mesh below `roots` (the entities); textured and rendered restore the content's materials */
export const applyShading = (roots: readonly Object3D[], mode: ShadingMode): void => {
  roots.forEach(root => root.traverse(object => {
    if (!(object instanceof Mesh) || object.name.startsWith("editor-")) return;
    const mesh = object as Mesh;
    if (mode === "wireframe" || mode === "solid") {
      if (!originals.has(mesh)) originals.set(mesh, mesh.material);
      mesh.material = mode === "wireframe" ? WIRE : SOLID;
    } else {
      const original = originals.get(mesh);
      if (original) {
        mesh.material = original;
        originals.delete(mesh);
      }
    }
  }));
};

/**
 * Outlines in the object's own space (Blender's selection): the edges of every mesh below `object` (a particle cloud:
 * its bounds) as lines that show through everything, added as children so they follow the object. `remove` takes
 * them away again.
 */
const OUTLINE_ANGLE = 40;

export interface Outline {
  remove(): void;
}

/** A particle cloud has no faces to outline: the box around its points stands in */
const boundsEdges = (geometry: BufferGeometry): EdgesGeometry => {
  geometry.computeBoundingBox();
  const box = geometry.boundingBox ?? new Box3();
  const size = box.getSize(new Vector3());
  const centre = box.getCenter(new Vector3());
  const cube = new BoxGeometry(size.x, size.y, size.z).translate(centre.x, centre.y, centre.z);
  const edges = new EdgesGeometry(cube);
  cube.dispose();
  return edges;
};

export const outline = (object: Object3D, color: number, width = 1, opacity = 0.95): Outline => {
  const lines: LineSegments[] = [];
  const material = new LineBasicMaterial({ color, depthTest: false, depthWrite: false, transparent: true, opacity, linewidth: width });
  object.traverse(child => {
    if (!(child instanceof Mesh || child instanceof Points) || child.name.startsWith("editor-")) return;
    const edges = new LineSegments(child instanceof Points ? boundsEdges(child.geometry) : new EdgesGeometry(child.geometry, OUTLINE_ANGLE), material);
    edges.name = "editor-outline";
    edges.renderOrder = 999;
    edges.frustumCulled = false;
    // Never hit by a pick: a line's ray threshold would make the outlined object catch clicks around it
    edges.raycast = () => {};
    child.add(edges);
    lines.push(edges);
  });
  return {
    remove: () => {
      lines.forEach(line => {
        line.removeFromParent();
        line.geometry.dispose();
      });
      material.dispose();
    },
  };
};

/** The active selection's orange (Blender's); the hover's neutral grey (what a click would pick), seen on the paper and on an entity alike */
export const SELECTION_COLOR = 0xffa028;
export const HOVER_COLOR = 0x8c8c8c;
