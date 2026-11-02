// The designer's thin client for POST /api/modules/compile (server/docs/API.md "Modules", #281). It takes the app's fetch
// function and the API base, never throws for an answer the API can give (403 creator_only, 422 compile_failed, 503 busy …):
// it returns them as values the panel can show – a muted sentence for "may not compile", not an error toast. The sources
// are loaded and saved by the app as the document `module:<id>` (ModuleStore, types.ts) – no member rows (#302).
import { readDiagnostics } from './diagnostics.ts';
import type { CompileRequest, CompileResult, Descriptor, Failure, FailureReason, WasmInfo } from './types.ts';

export interface DesignerApiOptions {
	/** the app's fetch (tests pass a fake) */
	fetch: typeof fetch;
	/** the API base without a trailing slash, e.g. `http://localhost:9000/api` */
	baseUrl: string;
	/** the session token for `Authorization: Bearer …`; null = the cookie alone */
	getToken?: () => string | null;
}

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

export function readWasm(raw: unknown): WasmInfo | undefined {
	if (typeof raw !== 'object' || raw === null) return undefined;
	const w = raw as Record<string, unknown>;
	if (typeof w.hash !== 'string') return undefined;
	return { hash: w.hash, bytes: Number(w.bytes) || 0, ...(typeof w.toolchain === 'string' ? { toolchain: w.toolchain } : {}), ...(typeof w.url === 'string' ? { url: w.url } : {}) };
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
		/** POST /modules/compile – creators only. `factory` writes the result (sources, descriptor, wasm hash) into a new draft of `module:<factory>` */
		async compile(request: CompileRequest): Promise<CompileResult> {
			const { status, body } = await call('POST', '/modules/compile', { files: request.files, ...(request.factory ? { factory: request.factory } : {}) });
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
					...(typeof body.module === 'string' ? { module: body.module } : {}),
					...(typeof body.version === 'number' ? { version: body.version } : {})
				};
			}
			if (status === 422 && body.error === 'compile_failed') {
				const f = failureOf(status, body);
				return { ok: false, reason: 'errors', errors: readDiagnostics(body.errors), message: f.message };
			}
			return failureOf(status, body);
		}
	};
}

export type DesignerApi = ReturnType<typeof createDesignerApi>;
