// workaround 16: a kind list with includes
export const audible = (n: { kind: string }) => ['osc', 'fm', 'noise'].includes(n.kind);
