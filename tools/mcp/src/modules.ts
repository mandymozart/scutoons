// compile_module (#449): a module saved through save_draft is compiled like the creator's Compile button – the same
// POST /api/modules/compile, here always the newest draft of `module:<id>`. The service writes the result (descriptor,
// wasm hash) into a new draft; publishing stays the member's click. These are the pure parts: what the API answers, as
// the agent should read it.
import { ApiError } from '@scutoons/api';

/** server ModuleSource::ID_PATTERN */
export const MODULE_ID = /^[a-z0-9_-]{1,32}$/;

export type CompileAnswer = {
	hash: string;
	descriptor: unknown;
	wasm: { hash: string; bytes: number; url?: string };
	warnings?: unknown[];
	module?: string;
	version?: number;
};

/** the request that compiles the newest draft of a module and stores the result in a new draft */
export const compileRequest = (id: string) => ({ factory: id });

/** a successful compile: what was built and the draft that holds it */
export function compiled(answer: CompileAnswer, open: string | null) {
	return {
		compiled: true,
		module: answer.module ?? null,
		version: answer.version ?? null,
		hash: answer.hash,
		bytes: answer.wasm.bytes,
		warnings: answer.warnings ?? [],
		descriptor: answer.descriptor,
		open
	};
}

/**
 * A refusal the agent fixes in the sources: the compiler's errors with file and line (422 compile_failed), the sources'
 * problems (422 invalid_module). Anything else is null: the caller's usual API error.
 */
export function compileRefused(error: unknown) {
	if (!(error instanceof ApiError) || error.status !== 422) return null;
	if (error.code === 'compile_failed') return { compiled: false, reason: 'the compiler refused the sources', errors: error.body?.errors ?? [] };
	if (error.code === 'invalid_module') return { compiled: false, reason: error.message, problems: error.body?.problems ?? [] };
	return null;
}
