// generate.ts (#372): every file written from the Rust layout or a schema, in one command.
//
//   npm run generate                 writes them (idempotent: a second run changes nothing)
//   npm run generate -- --check      only compares, exit 1 when one is stale – a step of `npm run check`
//
// The generators (each one is the owner of its files; this only runs them):
//   shared/live/src/protocol.ts, deck/protocol/src/generated.rs   deck/protocol/generate.ts, from deck/protocol/schema.json
//   shared/live-core/src/{wire,signals,clock-state}.ts            deck/live-core/src/bin/codecs-ts, from the layout tables
//                                                                 in wire.rs, signals.rs, clock.rs and deck-core's tempo
// The kind catalog's files (shared/live/src/catalog.ts, kinds.json …) are deck-space's `deck-schema` (deck:ci checks them).
import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const check = process.argv.includes('--check');
const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';

type Step = { name: string; cmd: string; args: string[]; cwd: string; env?: Record<string, string | undefined> };

export const steps = (check: boolean): Step[] => [
	{ name: 'protocol (schema.json → protocol.ts, generated.rs)', cmd: npm, args: ['run', 'generate', '-w', 'shared/live', ...(check ? ['--', '--check'] : [])], cwd: root },
	{ name: 'wire codecs (wire.rs, signals.rs, clock.rs → wire.ts, signals.ts, clock-state.ts)', cmd: 'cargo', args: ['run', '-q', '-p', 'live-core', '--features', 'codegen', '--bin', 'codecs-ts', ...(check ? ['--', '--check'] : [])], cwd: `${root}/deck` }
];

if (process.argv[1] === fileURLToPath(import.meta.url)) {
	for (const s of steps(check)) {
		console.log(`${check ? 'check' : 'generate'}: ${s.name}`);
		const r = spawnSync(s.cmd, s.args, { cwd: s.cwd, stdio: 'inherit', env: { ...process.env, ...s.env } as NodeJS.ProcessEnv, shell: process.platform === 'win32' });
		if (r.status !== 0) {
			console.error(`${check ? 'stale or failed' : 'failed'}: ${s.name}${check ? ' – run `npm run generate`' : ''}`);
			process.exit(r.status ?? 1);
		}
	}
	console.log(check ? 'generate: everything is current' : 'generate: done');
}
