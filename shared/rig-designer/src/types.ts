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
	unit?: string;
	doc?: string;
}

export type PortType = 'audio' | 'cv' | 'control' | 'video' | 'event';

export interface ModulePort {
	name: string;
	dir: 'in' | 'out';
	type: PortType;
	/** audio ports: 1 … 32 or "any" */
	channels?: number | 'any';
}

/** what a successful compile read from the wasm: the `deck.module` JSON as the module wrote it (module ABI v2, #343 –
 * deck-catalog's `Descriptor`, of which the designer shows these fields; deck-module `host::parse` validated it) */
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
	/** the CDN address; only in a compile's answer */
	url?: string;
}

/** a module as the document `module:<id>` stores it (server ModuleSource): `descriptor` and `wasm` come together, from a compile */
export interface StoredModule {
	name: string;
	files: ModuleFiles;
	descriptor?: Descriptor;
	wasm?: WasmInfo;
}

/** the compile request: the files as they are in the editor; `factory` writes the result into a new draft of `module:<factory>` */
export interface CompileRequest {
	files: ModuleFiles;
	factory?: string;
}

export interface CompiledModule {
	hash: string;
	descriptor: Descriptor;
	wasm: WasmInfo;
	warnings: Diagnostic[];
	/** the document the result was written into (`module:<id>`) and the draft version it made */
	module?: string;
	version?: number;
}

/** why a request did not work – the API's codes, grouped by what the panel does with them */
export type FailureReason =
	/** 403 creator_only: the user may not compile – a muted sentence, not an error */
	| 'creator_only'
	/** 401 */
	| 'signed_out'
	/** 503 live_show | build_busy | no_build_service | build_unavailable (the code in `code`) */
	| 'unavailable'
	/** 422 invalid_module / invalid_catalog (`problems`) */
	| 'invalid'
	/** the network, or an answer that is none of the above */
	| 'error';

export type CompileResult =
	| ({ ok: true } & CompiledModule)
	/** 422 compile_failed */
	| { ok: false; reason: 'errors'; errors: Diagnostic[]; message: string }
	| { ok: false; reason: FailureReason; code: string; message: string; problems?: string[] };

export type Failure = { ok: false; reason: FailureReason; code: string; message: string; problems?: string[] };

/**
 * Where the module's document lives – the app's (the creator: `/api/entities`, drafts, versions, publish). The designer
 * never knows the route; it loads and saves through this and the app keeps the version the next save builds on.
 */
export interface ModuleStore {
	/** the newest draft of `module:<id>`; `module: null` = none stored yet */
	load(id: string): Promise<{ ok: true; module: StoredModule | null } | Failure>;
	/** a new draft version of the document (`moduleContent`; the history keeps the old ones) */
	save(id: string, content: Record<string, unknown>): Promise<{ ok: true } | Failure>;
	/** a compile wrote a draft version itself (`factory`): the next save builds on it */
	compiled(id: string, version: number): void;
}
