// workaround 1 (BRIDGE-AUDIT.md §5): a TOML scene in a template literal – how FIXED_SCENE is emitted
const freq = 220;
export const SCENE = `[[node]]
id = "voice"
kind = "fm"
freq = ${freq}
`;
