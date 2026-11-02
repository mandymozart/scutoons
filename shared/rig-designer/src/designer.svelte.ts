// The Rust tab's state (runes): the files, the active one, compile / save with the API client, the problems, the
// console. The rules live in the tested pure modules (files, diagnostics, console, descriptor, reasons); this class
// only joins them to the reactive page. One instance per page, kept alive when the tab is hidden (the deck page never
// unmounts it – the engine under it keeps running).
import type { DesignerApi } from './api.ts';
import { ConsoleBuffer, type ConsoleLine } from './console.ts';
import { counts, jumpTarget, sortProblems, type JumpTarget } from './diagnostics.ts';
import { detailLines, headline } from './descriptor.ts';
import { addFile, fileOrder, fileProblems, freshName, isDirty, neighbour, removeFile, renameFile, setText, type FileProblem, type NameProblem } from './files.ts';
import { m } from './messages.ts';
import { failureText, isQuiet, problemText } from './reasons.ts';
import { STARTER_FILES } from './starter.ts';
import type { CompiledModule, Diagnostic, ModuleFiles } from './types.ts';

export class RigDesigner {
	files = $state<ModuleFiles>({ ...STARTER_FILES });
	/** what was last saved / loaded (the dirty flag compares against it) */
	saved = $state<ModuleFiles>({ ...STARTER_FILES });
	active = $state('lib.rs');
	/** the member's own module row (`m:<ulid>`) these files belong to; null = not saved yet */
	moduleId = $state<string | null>(null);
	name = $state('module');
	diagnostics = $state<Diagnostic[]>([]);
	compiled = $state<CompiledModule | null>(null);
	compiling = $state(false);
	saving = $state(false);
	loaded = $state(false);
	/** a reason the user may not compile (creator_only, signed_out): shown as a muted sentence, the actions disabled */
	blocked = $state<string>('');
	/** the console's lines (a copy of the buffer, for the page) */
	lines = $state<readonly ConsoleLine[]>([]);
	/** the last click on a problem: the editor follows it (the sequence number makes the same place count twice) */
	jump = $state<(JumpTarget & { seq: number }) | null>(null);
	/** the user's sentence about the last save / name edit that was refused */
	notice = $state('');

	private readonly buffer = new ConsoleBuffer();
	private seq = 0;

	constructor(private readonly api: DesignerApi) {}

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
	/** why Compile is disabled right now ('' = it is not): the reason shown as the button's title and the muted line */
	get compileReason(): string {
		if (this.compiling) return m['designer.compiling']();
		if (this.blocked) return failureText({ reason: this.blocked, code: this.blocked, message: '' });
		const p = this.limitProblems[0];
		return p ? problemText(p) : '';
	}
	get canCompile(): boolean {
		return !this.compiling && !this.saving && !this.blocked && this.limitProblems.length === 0;
	}
	get canSave(): boolean {
		return !this.compiling && !this.saving && this.limitProblems.length === 0 && this.blocked !== 'signed_out';
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

	/** the member's newest saved module, once, when the Rust tab first opens (a creator's own work comes back) */
	async load(): Promise<void> {
		if (this.loaded) return;
		this.loaded = true;
		const list = await this.api.list();
		if (!list.ok) {
			if (isQuiet(list.reason)) this.blocked = list.reason === 'signed_out' ? 'signed_out' : this.blocked;
			return;
		}
		const first = list.items[0];
		if (!first) return;
		const r = await this.api.load(first.id);
		if (!r.ok || this.dirty) return;
		this.adopt(r.module.id, r.module.name, r.module.files);
		this.log('info', m['designer.log_loaded']({ name: r.module.name || r.module.id }));
		if (r.module.descriptor) this.log('info', headline(r.module.descriptor));
	}

	private adopt(id: string, name: string, files: ModuleFiles): void {
		this.moduleId = id;
		this.name = name || 'module';
		this.files = files;
		this.saved = files;
		this.active = neighbour(files, 'lib.rs');
		this.diagnostics = [];
		this.compiled = null;
	}

	async save(): Promise<boolean> {
		if (!this.canSave) return false;
		this.saving = true;
		this.notice = '';
		try {
			const name = this.compiled?.descriptor.name ?? this.name;
			const r = this.moduleId ? await this.api.save(this.moduleId, name, this.files) : await this.api.create(name, this.files);
			if (!r.ok) {
				this.log('error', failureText(r) + (r.problems?.length ? ` ${r.problems.join('; ')}` : ''));
				if (isQuiet(r.reason)) this.blocked = r.reason;
				return false;
			}
			this.moduleId = r.module.id;
			this.name = r.module.name || name;
			this.saved = this.files;
			this.log('ok', m['designer.log_saved']({ id: r.module.id }));
			return true;
		} finally {
			this.saving = false;
		}
	}

	async compile(): Promise<boolean> {
		if (!this.canCompile) return false;
		this.compiling = true;
		this.diagnostics = [];
		this.log('info', m['designer.log_compiling']({ files: Object.keys(this.files).length }));
		try {
			const r = await this.api.compile({ files: this.files, ...(this.moduleId ? { member: this.moduleId } : {}) });
			if (r.ok) {
				this.compiled = r;
				this.diagnostics = r.warnings;
				if (this.moduleId) this.saved = this.files; // a compile into the member's module stores the sources too
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

	/** a fresh module from the starter */
	reset(): void {
		this.moduleId = null;
		this.name = 'module';
		this.files = { ...STARTER_FILES };
		this.saved = { ...STARTER_FILES };
		this.active = 'lib.rs';
		this.diagnostics = [];
		this.compiled = null;
	}
}
