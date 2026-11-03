// workaround 12: an LCG instead of Math.random
let s = 1;
export const next = () => ((s = (s * 1664525 + 1013904223) >>> 0), s / 4294967296);
