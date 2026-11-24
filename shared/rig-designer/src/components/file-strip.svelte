<script lang="ts">
	// The file tabs of the Rust tab: lib.rs · osc.rs · + file, the kit's pill tabs. Double-click (or F2) renames, the close
	// of the open tab removes (lib.rs stays), "+ file" asks for a name. Names are asked in the app's own dialogs (askDialog /
	// confirmDialog of @scutoons/ui), refusals come back as one muted sentence.
	import { askDialog, confirmDialog } from '@scutoons/ui';
	import { Button, PillTab } from '@scutoons/ui/editor';
	import type { RigDesigner } from '../designer.svelte.ts';
	import { MAIN_FILE } from '../files.ts';
	import { worstByFile } from '../diagnostics.ts';
	import { m } from '../messages.ts';
	import { nameProblemText } from '../reasons.ts';

	let { designer, onnotice }: { designer: RigDesigner; onnotice: (text: string) => void } = $props();

	const marks = $derived(worstByFile(designer.diagnostics));

	async function add() {
		const typed = await askDialog({
			title: m['designer.file_add_title'](),
			confirmLabel: m['designer.file_add'](),
			label: m['designer.file_name'](),
			value: designer.suggestName(),
			select: true
		});
		if (!typed) return;
		const problem = designer.add(typed);
		if (problem) onnotice(nameProblemText(problem));
	}

	async function rename(file: string) {
		// a factory module's files are read-only (#350)
		if (file === MAIN_FILE || designer.factory) return;
		const typed = await askDialog({
			title: m['designer.file_rename'](),
			confirmLabel: m['designer.file_rename'](),
			label: m['designer.file_name'](),
			value: file,
			select: true
		});
		if (!typed) return;
		const problem = designer.rename(file, typed);
		if (problem) onnotice(nameProblemText(problem));
	}

	async function remove(file: string) {
		const ok = await confirmDialog({ title: m['designer.file_remove_confirm']({ name: file }), confirmLabel: m['designer.file_remove']() });
		if (ok) designer.remove(file);
	}

	const removable = (file: string) => file === designer.active && file !== MAIN_FILE && !designer.factory;
</script>

<div class="files" role="group" aria-label={m['designer.files']()}>
	{#each designer.order as file (file)}
		<PillTab
			label={file}
			on={file === designer.active}
			title={file === MAIN_FILE || designer.factory ? file : m['designer.file_rename_title']()}
			closeLabel={m['designer.file_remove']()}
			onclose={removable(file) ? () => remove(file) : undefined}
			onclick={() => designer.setActive(file)}
			ondblclick={() => rename(file)}
			onkeydown={(e: KeyboardEvent) => e.key === 'F2' && rename(file)}
			data-testid="designer-file-{file}"
		>
			{#snippet trail()}{#if marks[file]}<i class="dot {marks[file]}" aria-hidden="true"></i>{/if}{/snippet}
		</PillTab>
	{/each}
	{#if !designer.factory}
		<Button variant="quiet" icon="plus" title={m['designer.file_add_title']()} onclick={add} data-testid="designer-file-add">{m['designer.file_add']()}</Button>
	{/if}
</div>

<style>
	.files {
		display: flex;
		align-items: center;
		gap: var(--kit-seam, 0.125rem);
		min-width: 0;
		overflow-x: auto;
		scrollbar-width: none;
	}
	/* a file's worst problem: the status colour, a small round mark after its name */
	.dot {
		flex: none;
		width: 0.4rem;
		height: 0.4rem;
		margin-left: var(--space-025);
		border-radius: 50%;
		background: var(--tool-warn);
	}
	.dot.error {
		background: var(--tool-stop);
	}
</style>
