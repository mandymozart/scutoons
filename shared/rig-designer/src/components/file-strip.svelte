<script lang="ts">
	// The file tabs of the Rust tab: lib.rs · osc.rs · + file. Double-click (or F2) renames, the × of the open tab
	// removes (lib.rs stays), "+ file" asks for a name. Names are asked in the app's own dialogs (askDialog /
	// confirmDialog of @scutoons/ui), refusals come back as one muted sentence.
	import { askDialog, confirmDialog, Icon } from '@scutoons/ui';
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
		if (file === MAIN_FILE) return;
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
</script>

<div class="files" role="tablist" aria-label={m['designer.files']()}>
	{#each designer.order as file (file)}
		<span class="file" class:on={file === designer.active}>
			<button
				type="button"
				role="tab"
				class="name"
				aria-selected={file === designer.active}
				title={file === MAIN_FILE ? file : m['designer.file_rename_title']()}
				data-testid="designer-file-{file}"
				onclick={() => designer.setActive(file)}
				ondblclick={() => rename(file)}
				onkeydown={(e) => e.key === 'F2' && rename(file)}
			>
				{file}{#if marks[file]}<i class="dot {marks[file]}" aria-hidden="true"></i>{/if}
			</button>
			{#if file === designer.active && file !== MAIN_FILE}
				<button type="button" class="x" title={m['designer.file_remove']()} aria-label={m['designer.file_remove']()} onclick={() => remove(file)}>
					<Icon name="x" />
				</button>
			{/if}
		</span>
	{/each}
	<button type="button" class="add" title={m['designer.file_add_title']()} onclick={add} data-testid="designer-file-add">+ {m['designer.file_add']()}</button>
</div>

<style>
	.files {
		display: flex;
		min-width: 0;
		overflow-x: auto;
		scrollbar-width: none;
	}
	.file {
		display: inline-flex;
		flex: none;
		border: var(--border-width-regular) solid var(--border-muted, color-mix(in srgb, var(--ink) 35%, transparent));
		margin-right: -2px;
	}
	.file.on {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	/* grouped tab buttons are square (DESIGN "Round vs square") at the control height */
	.name,
	.x,
	.add {
		height: var(--control-height);
		border: 0;
		border-radius: 0;
		background: none;
		color: inherit;
		font: inherit;
		font-family: var(--font-mono);
		font-size: var(--designer-ui-size, 0.75rem);
		cursor: pointer;
		white-space: nowrap;
	}
	.name {
		padding: 0 var(--space-075);
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
	}
	.x {
		display: inline-grid;
		place-items: center;
		width: var(--control-height);
		padding: 0;
	}
	.add {
		flex: none;
		margin-left: var(--space-05);
		padding: 0 var(--space-075);
		color: var(--text-muted);
	}
	.add:hover {
		color: var(--ink);
	}
	.dot {
		width: 0.5rem;
		height: 0.5rem;
		background: var(--state-warning);
	}
	.dot.error {
		background: var(--state-error);
	}
</style>
