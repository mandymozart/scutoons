#!/usr/bin/env node
// Starts the SCUTOONS Ask service (src/ask-main.ts) with Node's TypeScript support – the shared packages are TypeScript
// sources, nothing is built.
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const main = fileURLToPath(new URL('../src/ask-main.ts', import.meta.url));
const child = spawn(process.execPath, ['--experimental-transform-types', '--no-warnings', main, ...process.argv.slice(2)], { stdio: 'inherit' });
child.on('exit', (code, signal) => process.exit(signal ? 1 : (code ?? 0)));
for (const sig of ['SIGINT', 'SIGTERM']) process.on(sig, () => child.kill(sig));
