// exportScales – a bridge command module (#93; moved out of plugin/code.js 2026-10-20). Used by the storyboard scripts.
// a node (a slice frame) as PNGs at several scales: { node, scales } → [{ scale, base64 }]
// Plugin-sandbox syntax: no `?.`, no object spread.
const n = await figma.getNodeByIdAsync(args.node);
if (!n || !('exportAsync' in n)) throw new Error(`no node ${args.node}`);
const out = [];
for (const s of args.scales ?? [1]) out.push({ scale: s, base64: figma.base64Encode(await n.exportAsync({ format: 'PNG', constraint: { type: 'SCALE', value: s } })) });
lib.sweepPage(figma.currentPage);
return out;
