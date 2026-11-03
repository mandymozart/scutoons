// workaround 9: min / max with variable bounds
const LO = 20;
const HI = 2000;
export const freq = (x: number) => Math.min(HI, Math.max(LO, x));
