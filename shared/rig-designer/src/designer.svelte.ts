// The designer's state (runes): the module's files, the active one, compile / save, the problems, the console. The rules
// live in the tested pure modules (files, diagnostics, console, descriptor, store, reasons); this class only joins them to
// the reactive page. One instance per page; the module is a document `module:<id>` the app's ModuleStore loads and saves
// (drafts and history are the creator's own), the compile service writes the descriptor into it (`factory`).
import type { DesignerApi } from './api.ts';
import { ConsoleBuffer, type ConsoleLine } from './console.ts';
import { counts, jumpTarget, sortProblems } from './diagnostics.ts';
import { detailLines, headline } from './descriptor.ts';
import { addFile, fileOrder, fileProblems, freshName, isDirty, neighbour, removeFile, renameFile, setText, type FileProblem, type NameProblem } from './files.ts';
import { m } from './messages.ts';
import { failureText, isQuiet, problemText } from './reasons.ts';
import { STARTER_FILES } from './starter.ts';
import { moduleContent, sameFiles, type CompileOutcome } from './store.ts';
import type { Descriptor, Diagnostic, ModuleFiles, ModuleStore } from './types.ts';
import type { JumpTarget } from './diagnostics.ts';

export class RigDesigner {
	files = $state<ModuleFiles>({ ...STARTER_FILES });
	/** what was last saved / loaded (the dirty flag compares against it) */
	saved = $state<ModuleFiles>({ ...STARTER_FILES });
	active = $state('lib.rs');
	/** the module document (`module:<id>`) these files belong to; null = none open */
	moduleId = $state<string | null>(null);
	name = $state('');
	diagnostics = $state<Diagnostic[]>([]);
	/** the last compile (or the stored one): the sources it read, and what came out */
	outcome = $state<CompileOutcome | null>(null);
	compiling = $state(false);
	saving = $state(false);
	loading = $state(false);
	/** a reason the user may not compile (creator_only, signed_out): shown as a muted sentence, the actions disabled */
	blocked = $state<string>('');
	/** the console's lines (a copy of the buffer, for the page) */
	lines = $state<readonly ConsoleLine[]>([]);
	/** the last click on a problem: the editor follows it (the sequence number makes the same place count twice) */
	jump = $state<(JumpTarget & { seq: number }) | null>(null);
	/** the user's sentence about the last save / name edit that was refused */
	notice = $state('');
	/** the open module is a factory module (#350): read-only – no save, no edit; Compile only builds it once (the API: an admin) */
	factory = $state(false);
	/** the open module is a derivative of this one (#350); kept on every save */
	origin = $state<string | null>(null);

	private readonly buffer = new ConsoleBuffer();
	private seq = 0;

	constructor(
		private readonly api: DesignerApi,
		private readonly store: ModuleStore
	) {}

	get order(): string[] {
		return fileOrder(this.files);
	}
	get dirty(): boolean {
		return isDirty(this.saved, this.files);
	}
	get limitProblems(): FileProblem[] {
		return fileProblems(this.files);
	}
	get problems(): Diagnostic[] {
		return sortProblems(this.diagnostics, this.files);
	}
	get counts() {
		return counts(this.diagnostics);
	}
	/** the descriptor of the last compile (or the stored one) */
	get descriptor(): Descriptor | null {
		return this.outcome?.descriptor ?? null;
	}
	/** the files changed since the descriptor was read from them */
	get stale(): boolean {
		return this.outcome !== null && !sameFiles(this.outcome.files, this.files);
	}
	/** why Compile is disabled right now ('' = it is not): the reason shown as the button's title and the muted line */
	get compileReason(): string {
		if (this.compiling) return m['designer.compiling']();
		if (!this.moduleId) return m['designer.no_module']();
		if (this.blocked) return failureText({ reason: this.blocked, code: this.blocked, message: '' });
		// a built factory module: nothing to compile, Derive is the way to change it
		if (this.factory && this.outcome) return m['designer.reason_factory']();
		const p = this.limitProblems[0];
		return p ? problemText(p) : '';
	}
	get canCompile(): boolean {
		return !!this.moduleId && !this.compiling && !this.saving && !this.loading && !this.blocked && !(this.factory && this.outcome) && this.limitProblems.length === 0;
	}
	get canSave(): boolean {
		return !!this.moduleId && !this.factory && !this.compiling && !this.saving && !this.loading && this.limitProblems.length === 0 && this.blocked !== 'signed_out';
	}
	/** a factory module not built yet: the muted line says who builds it (the API decides: an admin) */
	get factoryNote(): string {
		return this.factory && !this.outcome ? m['designer.factory_build']() : '';
	}

	private log(level: ConsoleLine['level'], text: string): void {
		this.buffer.pushAlways(level, text);
		this.lines = this.buffer.lines;
	}

	clearConsole(): void {
		this.buffer.clear();
		this.lines = this.buffer.lines;
	}

	setActive(file: string): void {
		if (file in this.files) this.active = file;
	}

	edit(file: string, text: string): void {
		if (this.factory) return;
		this.files = setText(this.files, file, text);
	}

	/** "+ file": '' or the refusal */
	add(typed: string): NameProblem | '' {
		const r = addFile(this.files, typed);
		if (!r.ok) return r.problem;
		this.files = r.files;
		this.active = r.name;
		return '';
	}

	suggestName(): string {
		return freshName(this.files);
	}

	rename(from: string, typed: string): NameProblem | '' {
		const r = renameFile(this.files, from, typed);
		if (!r.ok) return r.problem;
		this.files = r.files;
		if (this.active === from) this.active = r.name;
		// the compiler's places named the old file
		this.diagnostics = this.diagnostics.map((d) => (d.file === from ? { ...d, file: r.name } : d));
		return '';
	}

	remove(file: string): NameProblem | '' {
		const r = removeFile(this.files, file);
		if (!r.ok) return r.problem;
		this.files = r.files;
		this.active = neighbour(this.files, this.active === file ? r.name : this.active);
		this.diagnostics = this.diagnostics.filter((d) => d.file !== file);
		return '';
	}

	/** a click on a problem: show the file and put the cursor there */
	goTo(d: Diagnostic): void {
		const t = jumpTarget(d, this.files);
		if (!t) return;
		this.active = t.file;
		this.jump = { ...t, seq: ++this.seq };
	}

	/** opens `module:<id>`: its newest draft, or – not stored yet – the starter, unsaved */
	async open(id: string | null): Promise<void> {
		this.moduleId = id;
		this.diagnostics = [];
		this.outcome = null;
		this.notice = '';
		this.factory = false;
		this.origin = null;
		// a factory module's refusal was that module's, not the user's
		if (this.blocked === 'factory') this.blocked = '';
		if (!id) return;
		this.loading = true;
		try {
			const r = await this.store.load(id);
			if (this.moduleId !== id) return; // another module was opened meanwhile
			if (!r.ok) {
				if (isQuiet(r.reason)) this.blocked = r.reason;
				this.log('error', failureText(r));
				return;
			}
			if (!r.module) {
				this.name = id;
				this.files = { ...STARTER_FILES };
				this.saved = {};
				this.active = 'lib.rs';
				return;
			}
			this.name = r.module.name;
			this.factory = r.module.factory === true;
			this.origin = r.module.origin ?? null;
			this.files = r.module.files;
			this.saved = r.module.files;
			this.active = neighbour(r.module.files, 'lib.rs');
			if (r.module.descriptor && r.module.wasm) {
				this.outcome = { files: r.module.files, descriptor: r.module.descriptor, wasm: r.module.wasm };
				this.log('info', headline(r.module.descriptor));
			}
			this.log('info', m['designer.log_loaded']({ name: id }));
		} finally {
			this.loading = false;
		}
	}

	async save(): Promise<boolean> {
		const id = this.moduleId;
		if (!id || !this.canSave) return false;
		this.saving = true;
		this.notice = '';
		try {
			const r = await this.store.save(id, moduleContent(id, this.name || id, this.files, this.outcome, this.origin));
			if (!r.ok) {
				this.log('error', failureText(r) + (r.problems?.length ? ` ${r.problems.join('; ')}` : ''));
				if (isQuiet(r.reason)) this.blocked = r.reason;
				return false;
			}
			this.saved = this.files;
			this.log('ok', m['designer.log_saved']({ id }));
			return true;
		} finally {
			this.saving = false;
		}
	}

	async compile(): Promise<boolean> {
		const id = this.moduleId;
		if (!id || !this.canCompile) return false;
		this.compiling = true;
		this.diagnostics = [];
		const files = this.files;
		this.log('info', m['designer.log_compiling']({ files: Object.keys(files).length }));
		try {
			// `factory`: the service writes the sources, the descriptor and the hash into a new draft of module:<id> itself
			// a factory module builds from its stored sources, unchanged (#350)
			const r = await this.api.compile(this.factory ? { factory: id } : { files, factory: id });
			if (r.ok) {
				this.outcome = { files, descriptor: r.descriptor, wasm: r.wasm };
				if (r.version !== undefined) {
					this.store.compiled(id, r.version);
					this.saved = files; // the compile stored the sources too
				}
				this.diagnostics = r.warnings;
				// the build log reads top down: warnings, the descriptor's parameters and ports, then the verdict and the summary
				for (const w of r.warnings) this.log('warn', `${w.file}:${w.line} ${w.message}`);
				for (const line of detailLines(r.descriptor)) this.log('info', line);
				this.log('ok', m['designer.log_ok']({ hash: r.hash.slice(0, 12), bytes: r.wasm.bytes }));
				this.log('ok', headline(r.descriptor));
				return true;
			}
			if (r.reason === 'errors') {
				this.diagnostics = r.errors;
				const c = counts(r.errors);
				this.log('error', m['designer.log_failed']({ errors: c.errors, warnings: c.warnings }));
				return false;
			}
			if (isQuiet(r.reason)) this.blocked = r.reason;
			this.log(isQuiet(r.reason) ? 'info' : 'error', m['designer.log_blocked']({ reason: failureText(r) }));
			return false;
		} finally {
			this.compiling = false;
		}
	}
}
