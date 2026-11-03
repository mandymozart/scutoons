// workaround 7: a ternary clamp
export const index = (x: number) => (x < 0 ? 0 : x > 8 ? 8 : x);
