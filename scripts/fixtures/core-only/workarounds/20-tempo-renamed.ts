// workaround 20 (caught by v1 too): bpm under another name with / 60000
export const beats = (ms: number, quarterRate: number) => (ms / 60000) * quarterRate;
