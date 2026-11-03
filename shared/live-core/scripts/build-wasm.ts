// `npm run build:wasm -w shared/live-core`: live_core.wasm for wasm32 + simd128 (scripts/wasm.ts says where and how).
import { execFileSync } from 'node:child_process';
import { buildCommand, deckDir, wasmPath } from './wasm.ts';

const { cmd, args, env } = buildCommand();
execFileSync(cmd, args, { stdio: 'inherit', cwd: deckDir, env });
console.log(`→ ${wasmPath()}`);
