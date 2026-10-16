// Types of record-light.js (plain JS: three.js has no bundled types and the repo carries no @types/three).

/** a soft contact shadow w × d on the floor */
export function contactShadow(THREE: any, w: number, d: number, opacity?: number): any;
/** the spot, the floor, the plinth's contact shadow */
export function moodyLights(THREE: any, renderer: any, scene: any, deck: { x: number; z: number; w: number; d: number }, table?: string | null): void;
/** these cast (and take) the spot's shadow */
export function castShadows(...objs: any[]): void;
