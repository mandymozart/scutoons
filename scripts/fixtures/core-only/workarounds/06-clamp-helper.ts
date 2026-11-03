// workaround 6: a clamp helper with the catalog's bounds
const clamp = (x: number, lo: number, hi: number) => (x < lo ? lo : x > hi ? hi : x);
export const ratio = (x: number) => clamp(x, 0.5, 4);
