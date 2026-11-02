// The shapes of the module API (server/docs/API.md "Modules", #281) as the designer uses them.

/** a module's sources: `{ "lib.rs": "…", "osc.rs": "…" }` */
export type ModuleFiles = Record<string, string>;

/** one compiler message – the fields the API keeps (ModuleCompiler::diagnostics) */
export interface Diagnostic {
	file: string;
	/** 1-based; 0 = the message names no place */
	line: number;
	/** 1-based */
	column: number;
	/** "error" | "warning" | "note" … as rustc says */
	level: string;
	message: string;
}

export interface ModuleParam {
	name: string;
	min: number;
	max: number;
	default: number;
}

export type PortKind = 'audio' | 'control' | 'video' | 'event';

export interface ModulePort {
	name: string;
	dir: 'in' | 'out';
	kind: PortKind;
}

/** what a successful compile read from the wasm (deck-module `host::Descriptor`) */
export interface Descriptor {
	abi: number;
	name: string;
	version: string;
	params: ModuleParam[];
	ports: ModulePort[];
}

export interface WasmInfo {
	hash: string;
	bytes: number;
	toolchain?: string;
	url: string;
}

/** a member's own module row (GET /modules/{m:id}) */
export interface MemberModule {
	id: string;
	name: string;
	files: ModuleFiles;
	descriptor?: Descriptor;
	wasm?: WasmInfo;
}

/** the compile request: the files as they are in the editor, optionally written into the member's own module */
export interface CompileRequest {
	files: ModuleFiles;
	member?: string;
}

export interface CompiledModule {
	hash: string;
	descriptor: Descriptor;
	wasm: WasmInfo;
	warnings: Diagnostic[];
	/** the member module the result was written into */
	module?: string;
}

/** why a request did not work – the API's codes, grouped by what the panel does with them */
export type FailureReason =
	/** 403 creator_only: the user may not compile – a muted sentence, not an error */
	| 'creator_only'
	/** 401 */
	| 'signed_out'
	/** 503 live_show | build_busy | no_build_service | build_unavailable (the code in `code`) */
	| 'unavailable'
	/** 422 invalid_module / too_many_modules (`problems`) */
	| 'invalid'
	/** the network, or an answer that is none of the above */
	| 'error';

export type CompileResult =
	| ({ ok: true } & CompiledModule)
	/** 422 compile_failed */
	| { ok: false; reason: 'errors'; errors: Diagnostic[]; message: string }
	| { ok: false; reason: FailureReason; code: string; message: string; problems?: string[] };

export type SaveResult = { ok: true; module: MemberModule } | { ok: false; reason: FailureReason; code: string; message: string; problems?: string[] };
export type LoadResult = { ok: true; module: MemberModule } | { ok: false; reason: FailureReason; code: string; message: string };
export type ListResult = { ok: true; items: { id: string; name: string; descriptor?: Descriptor }[] } | { ok: false; reason: FailureReason; code: string; message: string };
