// workaround 8: max over min (the v1 regex wants a literal first)
export const level = (x: number) => Math.max(Math.min(x, 1), 0);
