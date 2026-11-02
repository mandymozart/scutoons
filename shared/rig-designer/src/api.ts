// The designer's thin client for /api/modules (server/docs/API.md "Modules", #281). It takes the app's fetch function and
// the API base, never throws for an answer the API can give (403 creator_only, 422 compile_failed, 503 busy …): it
// returns them as values the panel can show – a muted sentence for "may not compile", not an error toast.
import { readDiagnostics } from './diagnostics.ts';
import type {
	CompileRequest,
	CompileResult,
	Descriptor,
	FailureReason,
	ListResult,
	LoadResult,
	MemberModule,
	ModuleFiles,
	SaveResult,
	WasmInfo
} from './types.ts';

export interface DesignerApiOptions {
	/** the app's fetch (tests pass a fake) */
	fetch: typeof fetch;
	/** the API base without a trailing slash, e.g. `http://localhost:9000/api` */
	baseUrl: string;
	/** the session token for `Authorization: Bearer …`; null = the cookie alone */
	getToken?: () => string | null;
}

type Failure = { ok: false; reason: FailureReason; code: string; message: string; problems?: string[] };

const UNAVAILABLE = new Set(['live_show', 'build_busy', 'no_build_service', 'build_unavailable']);

/** an API error answer as a failure the panel understands */
export function failureOf(status: number, body: Record<string, unknown>): Failure {
	const code = typeof body.error === 'string' ? body.error : status === 0 ? 'network' : `http_${status}`;
	const message = typeof body.message === 'string' && body.message !== '' ? body.message : code;
	const problems = Array.isArray(body.problems) ? body.problems.filter((p): p is string => typeof p === 'string') : undefined;
	let reason: FailureReason = 'error';
	if (status === 403 && code === 'creator_only') reason = 'creator_only';
	else if (status === 401) reason = 'signed_out';
	else if (UNAVAILABLE.has(code)) reason = 'unavailable';
	else if (status === 422) reason = 'invalid';
	return { ok: false, reason, code, message, ...(problems ? { problems } : {}) };
}

function readWasm(raw: unknown): WasmInfo | undefined {
	if (typeof raw !== 'object' || raw === null) return undefined;
	const w = raw as Record<string, unknown>;
	if (typeof w.hash !== 'string') return undefined;
	return { hash: w.hash, bytes: Number(w.bytes) || 0, ...(typeof w.toolchain === 'string' ? { toolchain: w.toolchain } : {}), url: typeof w.url === 'string' ? w.url : '' };
}

function readModule(raw: unknown): MemberModule | null {
	if (typeof raw !== 'object' || raw === null) return null;
	const r = raw as Record<string, unknown>;
	if (typeof r.id !== 'string') return null;
	const files: ModuleFiles = {};
	if (typeof r.files === 'object' && r.files !== null) for (const [k, v] of Object.entries(r.files)) if (typeof v === 'string') files[k] = v;
	const wasm = readWasm(r.wasm);
	return {
		id: r.id,
		name: typeof r.name === 'string' ? r.name : '',
		files,
		...(typeof r.descriptor === 'object' && r.descriptor !== null ? { descriptor: r.descriptor as Descriptor } : {}),
		...(wasm ? { wasm } : {})
	};
}

export function createDesignerApi(o: DesignerApiOptions) {
	const base = o.baseUrl.replace(/\/+$/, '');

	async function call(method: string, path: string, body?: unknown): Promise<{ status: number; body: Record<string, unknown> }> {
		const headers: Record<string, string> = { Accept: 'application/json' };
		if (body !== undefined) headers['Content-Type'] = 'application/json';
		const token = o.getToken?.();
		if (token) headers.Authorization = `Bearer ${token}`;
		try {
			const res = await o.fetch(base + path, { method, headers, credentials: 'include', ...(body !== undefined ? { body: JSON.stringify(body) } : {}) });
			const parsed: unknown = await res.json().catch(() => ({}));
			return { status: res.status, body: typeof parsed === 'object' && parsed !== null ? (parsed as Record<string, unknown>) : {} };
		} catch (e) {
			return { status: 0, body: { error: 'network', message: (e as Error).message || 'network' } };
		}
	}

	return {
		/** POST /modules/compile – creators only. `member` writes the result into that own module (and replaces its sources) */
		async compile(request: CompileRequest): Promise<CompileResult> {
			const { status, body } = await call('POST', '/modules/compile', { files: request.files, ...(request.member ? { member: request.member } : {}) });
			if (status === 200) {
				const wasm = readWasm(body.wasm);
				const descriptor = body.descriptor as Descriptor | undefined;
				if (!wasm || !descriptor || typeof body.hash !== 'string') return { ok: false, reason: 'error', code: 'bad_answer', message: 'bad_answer' };
				return {
					ok: true,
					hash: body.hash,
					descriptor,
					wasm,
					warnings: readDiagnostics(body.warnings),
					...(typeof body.module === 'string' ? { module: body.module } : {})
				};
			}
			if (status === 422 && body.error === 'compile_failed') {
				const f = failureOf(status, body);
				return { ok: false, reason: 'errors', errors: readDiagnostics(body.errors), message: f.message };
			}
			return failureOf(status, body);
		},

		/** GET /modules – the member's own, without sources */
		async list(): Promise<ListResult> {
			const { status, body } = await call('GET', '/modules');
			if (status !== 200) return failureOf(status, body);
			const items = Array.isArray(body.items) ? body.items.map(readModule).filter((m): m is MemberModule => m !== null) : [];
			return { ok: true, items: items.map((m) => ({ id: m.id, name: m.name, ...(m.descriptor ? { descriptor: m.descriptor } : {}) })) };
		},

		/** GET /modules/{id} – an own `m:<ulid>` */
		async load(id: string): Promise<LoadResult> {
			const { status, body } = await call('GET', `/modules/${encodeURIComponent(id)}`);
			const module = status === 200 ? readModule(body.module) : null;
			return module ? { ok: true, module } : failureOf(status, body);
		},

		/** POST /modules – a new row of the member's */
		async create(name: string, files: ModuleFiles): Promise<SaveResult> {
			const { status, body } = await call('POST', '/modules', { name, files });
			const module = status === 201 ? readModule(body.module) : null;
			return module ? { ok: true, module } : failureOf(status, body);
		},

		/** PUT /modules/{m:id} – other sources drop the old descriptor / wasm */
		async save(id: string, name: string, files: ModuleFiles): Promise<SaveResult> {
			const { status, body } = await call('PUT', `/modules/${encodeURIComponent(id)}`, { name, files });
			const module = status === 200 ? readModule(body.module) : null;
			return module ? { ok: true, module } : failureOf(status, body);
		},

		/** DELETE /modules/{m:id} */
		async remove(id: string): Promise<{ ok: true } | Failure> {
			const { status, body } = await call('DELETE', `/modules/${encodeURIComponent(id)}`);
			return status === 200 ? { ok: true } : failureOf(status, body);
		}
	};
}

export type DesignerApi = ReturnType<typeof createDesignerApi>;
