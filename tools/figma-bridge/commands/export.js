// export – a bridge command module (#93; moved out of plugin/code.js 2026-10-20). Used by cli.mjs export · the SCUTOONS MCP figma_view.
// Plugin-sandbox syntax: no `?.`, no object spread.
const node = await lib.nodeOrPage(args);
if (!('exportAsync' in node)) throw new Error(`${node.type} can't be exported`);
const format = (args.format ?? 'PNG').toUpperCase();
const settings = format === 'SVG' ? { format: 'SVG' } : { format, constraint: { type: 'SCALE', value: args.scale ?? 1 } };
const bytes = await node.exportAsync(settings);
lib.sweepPage(figma.currentPage);
return { name: node.name, format, bytes: bytes.length, base64: figma.base64Encode(bytes) };
