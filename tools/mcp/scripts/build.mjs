#!/usr/bin/env node
// The member's copy of the SCUTOONS MCP server (agents/MCP.md §3a): ONE file that needs only Node ≥ 22 – no checkout,
// no npm install – and the Claude Desktop extension around it. The format rules (resources.ts reads them from the
// source files at run time) are read HERE and embedded; the API defaults to production (config.ts BUNDLED).
//
//   node tools/mcp/scripts/build.mjs [outDir]        default: server/mcp – shipped with the API (issue 76, owner
//                                                     2026-10-20 "den MCP vom Server aus mitgeben"): the api deploy
//                                                     runs it into its staging copy; GET /api/mcp/download serves it
// Writes <outDir>/scutoons-mcp.mjs (Claude Code: `node scutoons-mcp.mjs`), <outDir>/scutoons.mcpb (Claude Desktop:
// a zip – manifest.json + server/index.mjs; double-click, paste the token, pick the workspace folder),
// <outDir>/scutoons-figma-plugin.zip (the SCUTOONS Bridge plugin for Figma desktop, #127 – its manifest, code.js, ui.html;
// the command modules travel inside the MCP, whose relay sends them) and <outDir>/scutoons-mcp.json
// { version, git_sha, sha256, sha256_mcpb, sha256_figma_plugin, figma_plugin_version, built_at } (GET /api/mcp/version;
// the bundle compares itself with it).
import { mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { crc32, deflateRawSync } from 'node:zlib';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { rolldown } from 'rolldown';

const HERE = dirname(fileURLToPath(import.meta.url));
const PKG = resolve(HERE, '..');
const ROOT = resolve(PKG, '../..');
const OUT = resolve(process.cwd(), process.argv[2] ?? resolve(ROOT, 'server/mcp'));
const { version } = JSON.parse(readFileSync(resolve(PKG, 'package.json'), 'utf8'));

// 1. the rules, read now (Node's type stripping runs resources.ts like the server does)
const rules = JSON.parse(
	execFileSync(process.execPath, ['--experimental-transform-types', '--no-warnings', '--input-type=module', '-e',
		`const { RULES } = await import(${JSON.stringify(pathToFileURL(resolve(PKG, 'src/resources.ts')).href)});
		 process.stdout.write(JSON.stringify(RULES.map((r) => ({ name: r.name, uri: r.uri, title: r.title, description: r.description, body: r.text() }))));`
	], { maxBuffer: 64 * 1024 * 1024 }).toString('utf8')
);

// the Figma bridge's command modules (tools/figma-bridge/commands/*.js) – the MCP's own relay sends them to the plugin
const BRIDGE = resolve(ROOT, 'tools/figma-bridge');
const commands = Object.fromEntries(
	readdirSync(resolve(BRIDGE, 'commands'))
		.filter((f) => /^[a-z][\w-]*\.js$/.test(f))
		.sort()
		.map((f) => [f.slice(0, -3), readFileSync(resolve(BRIDGE, 'commands', f), 'utf8')])
);

// 2. one ES module: resources.ts replaced by the embedded rules, figma-commands.ts by the embedded modules
const RESOURCES = resolve(PKG, 'src/resources.ts');
const COMMANDS = resolve(PKG, 'src/figma-commands.ts');
const bundle = await rolldown({
	input: resolve(PKG, 'src/main.ts'),
	platform: 'node',
	transform: { define: { __SCUTOONS_BUNDLE__: 'true' } },
	plugins: [
		{
			name: 'embedded-rules',
			resolveId(source, importer) {
				const path = importer ? resolve(dirname(importer), source) : null;
				if (path === RESOURCES) return '\0scutoons-rules';
				if (path === COMMANDS) return '\0scutoons-figma-commands';
				return null;
			},
			load(id) {
				if (id === '\0scutoons-rules') return `export const RULES = ${JSON.stringify(rules)}.map((r) => ({ ...r, text: () => r.body }));`;
				if (id === '\0scutoons-figma-commands')
					return `const C = ${JSON.stringify(commands)};\nexport const COMMANDS_DIR = null;\nexport function commandSource(name) { return Object.hasOwn(C, name) ? C[name] : null; }`;
				return null;
			}
		}
	]
});
const { output } = await bundle.generate({
	format: 'esm',
	codeSplitting: false,
	banner: `// SCUTOONS MCP server ${version} – the member's own AI with their rights (https://scutoons.com/account → AI connections). Built from tools/mcp.\nimport { createRequire as __scutoonsRequire } from 'node:module';\nconst require = __scutoonsRequire(import.meta.url);`
});
await bundle.close();
const code = output[0].code;

// 3. the extension: a zip with manifest.json + server/index.mjs (MCPB manifest 0.3)
const manifest = {
	manifest_version: '0.3',
	name: 'scutoons',
	display_name: 'SCUTOONS',
	version,
	description: 'Work in SCUTOONS with your own rights: read and check toon content and its story flow, save drafts, the Embassy and the admin data (by your roles). You publish, send and apply in the apps.',
	author: { name: 'SCUTOONS', url: 'https://scutoons.com' },
	homepage: 'https://scutoons.com/account',
	server: {
		type: 'node',
		entry_point: 'server/index.mjs',
		mcp_config: {
			command: 'node',
			args: ['${__dirname}/server/index.mjs'],
			env: { SCUTOONS_TOKEN: '${user_config.token}', SCUTOONS_WORKSPACE: '${user_config.workspace}', COMFY_URL: '${user_config.comfy_url}' }
		}
	},
	user_config: {
		token: {
			type: 'string',
			title: 'Your SCUTOONS token',
			description: 'Create it on scutoons.com/account → AI connections (sct_…). Revoke it there any time.',
			sensitive: true,
			required: true
		},
		workspace: {
			type: 'directory',
			title: 'Workspace folder',
			description: 'Where generated stills and videos are saved (<toon>/<chapter>/<shot>--g<n>.png). Optional – needed for check_generation and generate_local.',
			required: false
		},
		comfy_url: {
			type: 'string',
			title: 'ComfyUI address',
			description: 'Only if you run ComfyUI on this computer for generate_local (free stills on your own GPU).',
			default: 'http://127.0.0.1:8188',
			required: false
		}
	},
	compatibility: { platforms: ['darwin', 'win32', 'linux'], runtimes: { node: '>=22.0.0' } }
};

/** a zip (deflate) of { name: Buffer } – .mcpb is a plain zip */
function zip(files) {
	const local = [];
	const central = [];
	let offset = 0;
	for (const [name, data] of Object.entries(files)) {
		const nameBuf = Buffer.from(name, 'utf8');
		const packed = deflateRawSync(data);
		const crc = crc32(data);
		const head = Buffer.alloc(30);
		head.writeUInt32LE(0x04034b50, 0);
		head.writeUInt16LE(20, 4);
		head.writeUInt16LE(0x0800, 6); // UTF-8 names
		head.writeUInt16LE(8, 8); // deflate
		head.writeUInt32LE(0, 10); // time / date
		head.writeUInt32LE(crc >>> 0, 14);
		head.writeUInt32LE(packed.length, 18);
		head.writeUInt32LE(data.length, 22);
		head.writeUInt16LE(nameBuf.length, 26);
		head.writeUInt16LE(0, 28);
		local.push(head, nameBuf, packed);
		const dir = Buffer.alloc(46);
		dir.writeUInt32LE(0x02014b50, 0);
		dir.writeUInt16LE(20, 4);
		dir.writeUInt16LE(20, 6);
		dir.writeUInt16LE(0x0800, 8);
		dir.writeUInt16LE(8, 10);
		dir.writeUInt32LE(0, 12);
		dir.writeUInt32LE(crc >>> 0, 16);
		dir.writeUInt32LE(packed.length, 20);
		dir.writeUInt32LE(data.length, 24);
		dir.writeUInt16LE(nameBuf.length, 28);
		dir.writeUInt32LE(offset, 42);
		central.push(dir, nameBuf);
		offset += head.length + nameBuf.length + packed.length;
	}
	const dirSize = central.reduce((n, b) => n + b.length, 0);
	const end = Buffer.alloc(22);
	end.writeUInt32LE(0x06054b50, 0);
	end.writeUInt16LE(Object.keys(files).length, 8);
	end.writeUInt16LE(Object.keys(files).length, 10);
	end.writeUInt32LE(dirSize, 12);
	end.writeUInt32LE(offset, 16);
	return Buffer.concat([...local, ...central, end]);
}

const sha256 = (data) => createHash('sha256').update(data).digest('hex');
let gitSha = process.env.GITHUB_SHA || '';
try {
	gitSha ||= execFileSync('git', ['rev-parse', 'HEAD'], { cwd: ROOT }).toString('utf8').trim();
} catch {
	gitSha ||= 'dev';
}
const codeBuf = Buffer.from(code);
const json = (value) => JSON.stringify(value, null, '\t') + '\n';
const mcpb = zip({ 'manifest.json': Buffer.from(json(manifest)), 'server/index.mjs': codeBuf });

// 4. the Figma plugin (#127): the files Figma's "Import plugin from manifest…" reads, as they are in the checkout – only
// the UI's hint names the MCP instead of the checkout's supervisor (the plugin's code and protocol are unchanged)
const plugin = (f) => readFileSync(resolve(BRIDGE, 'plugin', f));
const pluginUi = plugin('ui.html').toString('utf8').replace('node tools/figma-bridge/supervise.mjs --detach', 'your SCUTOONS MCP (ask Claude: figma_status)');
const pluginVersion = /const VERSION = '([^']+)'/.exec(plugin('code.js').toString('utf8'))?.[1] ?? null;
const pluginReadme = [
	`SCUTOONS Bridge – the Figma plugin of the SCUTOONS MCP ${version} (plugin code ${pluginVersion})`,
	'',
	'1. Figma desktop (not the browser) -> open any design file -> Plugins -> Development -> Import plugin from manifest...',
	'   -> pick manifest.json in this folder. Keep the folder where it is (Figma loads the plugin from here).',
	'2. Run it in the file you work on: Plugins -> Development -> SCUTOONS Bridge (afterwards Ctrl+Alt+P / Cmd+Option+P).',
	'   The small window shows a green dot when it is connected to your SCUTOONS MCP.',
	'3. Check: ask your Claude "whoami" (kit -> figma) or "figma_status".',
	'',
	'A newer SCUTOONS MCP may bring a newer plugin: download https://scutoons.com/figma-plugin again and replace these files.',
	''
].join('\r\n');
const figmaZip = zip({
	'scutoons-figma-plugin/manifest.json': plugin('manifest.json'),
	'scutoons-figma-plugin/code.js': plugin('code.js'),
	'scutoons-figma-plugin/ui.html': Buffer.from(pluginUi),
	'scutoons-figma-plugin/README.txt': Buffer.from(pluginReadme)
});

mkdirSync(OUT, { recursive: true });
writeFileSync(resolve(OUT, 'scutoons-mcp.mjs'), codeBuf);
writeFileSync(resolve(OUT, 'scutoons.mcpb'), mcpb);
writeFileSync(resolve(OUT, 'scutoons-figma-plugin.zip'), figmaZip);
writeFileSync(
	resolve(OUT, 'scutoons-mcp.json'),
	json({
		version,
		git_sha: gitSha,
		sha256: sha256(codeBuf),
		sha256_mcpb: sha256(mcpb),
		sha256_figma_plugin: sha256(figmaZip),
		figma_plugin_version: pluginVersion,
		built_at: new Date().toISOString()
	})
);
console.log(
	`scutoons-mcp ${version}: ${OUT} – scutoons-mcp.mjs (${Math.round(code.length / 1024)} KB), scutoons.mcpb, scutoons-figma-plugin.zip (plugin ${pluginVersion}), scutoons-mcp.json, ${rules.length} rules + ${Object.keys(commands).length} Figma command modules embedded`
);
