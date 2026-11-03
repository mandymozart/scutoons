// workaround 17: a map lookup on the kind
export const busOf = (n: { kind: string }, a: number) => ({ mixer: a, bus: a * 2 } as Record<string, number>)[n.kind];
