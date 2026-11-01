// The rig editor's logic (#269, agents/CATALOG.md "Instruments, presets, rigs"): the validator mirroring the server's
// RigValidator over schema.json (generated from deck-space's Rust types), the graph rules, and the forms' helpers.
export * from './schema.ts';
export * from './graph.ts';
export * from './validate.ts';
export * from './edit.ts';
export * from './problems.ts';
