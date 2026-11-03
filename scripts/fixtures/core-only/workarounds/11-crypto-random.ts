// workaround 11: randomness from crypto
export const seed = () => crypto.getRandomValues(new Uint32Array(1))[0];
