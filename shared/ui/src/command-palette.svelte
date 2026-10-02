<script lang="ts" module>
	export interface Command {
		id: string;
		label: string;
		/** one-line description / target, shown muted */
		hint?: string;
		/** heading the command is listed under */
		group?: string;
		/** extra words the filter matches */
		keywords?: string;
		run: () => void;
	}

	/** Every word of the query must appear in label, hint, group or keywords (case-insensitive) */
	export function matchCommand(command: Command, query: string): boolean {
		const hay = `${command.label} ${command.hint ?? ''} ${command.group ?? ''} ${command.keywords ?? ''}`.toLowerCase();
		return query
			.toLowerCase()
			.split(/\s+/)
			.filter(Boolean)
			.every((word) => hay.includes(word));
	}
</script>

<script lang="ts">
	/**
	 * Command palette on the native <dialog>: Ctrl+K / ⌘K anywhere, `/` outside text fields (`hotkeys`), arrow keys,
	 * Enter runs, Escape closes. `commands` are filtered by the query; `dynamic(query)` adds commands that depend on
	 * the query (e.g. "Open code ABC"), listed first.
	 */
	let {
		open = $bindable(false),
		commands,
		dynamic,
		hotkeys = true,
		label = 'Command palette',
		placeholder = 'Type a command or search …',
		emptyLabel = 'No matching command.',
		hint = '↑↓ to move · Enter to run · Esc to close'
	}: {
		open?: boolean;
		commands: Command[];
		dynamic?: (query: string) => Command[];
		hotkeys?: boolean;
		label?: string;
		placeholder?: string;
		emptyLabel?: string;
		hint?: string;
	} = $props();

	const uid = $props.id();
	let dialog: HTMLDialogElement | null = $state(null);
	let input: HTMLInputElement | null = $state(null);
	let query = $state('');
	let index = $state(0);

	const results = $derived([
		...(dynamic?.(query.trim()) ?? []),
		...commands.filter((command) => matchCommand(command, query))
	]);

	$effect(() => {
		if (!dialog) return;
		if (open && !dialog.open) {
			query = '';
			index = 0;
			dialog.showModal();
			input?.focus();
		} else if (!open && dialog.open) dialog.close();
	});

	$effect(() => {
		if (!hotkeys) return;
		const onKey = (event: KeyboardEvent) => {
			// the target can be the window itself (no `closest`)
			const target = event.target instanceof Element ? event.target : null;
			const typing = !!target?.closest('input, textarea, select, [contenteditable=""], [contenteditable="true"]');
			if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
				event.preventDefault();
				open = !open;
			} else if (event.key === '/' && !typing && !open && !event.ctrlKey && !event.metaKey && !event.altKey) {
				event.preventDefault();
				open = true;
			}
		};
		window.addEventListener('keydown', onKey);
		return () => window.removeEventListener('keydown', onKey);
	});

	function run(command: Command | undefined) {
		if (!command) return;
		open = false;
		command.run();
	}

	function keydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown') {
			event.preventDefault();
			index = results.length ? (index + 1) % results.length : 0;
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			index = results.length ? (index - 1 + results.length) % results.length : 0;
		} else if (event.key === 'Enter') {
			event.preventDefault();
			run(results[index]);
		}
	}

	$effect(() => {
		// keep the active option visible
		document.getElementById(`${uid}-option-${index}`)?.scrollIntoView({ block: 'nearest' });
	});

	function backdrop(event: MouseEvent) {
		if (event.target === dialog) open = false;
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog bind:this={dialog} class="sc-palette" aria-label={label} onclose={() => (open = false)} onclick={backdrop}>
	<input
		bind:this={input}
		bind:value={query}
		oninput={() => (index = 0)}
		onkeydown={keydown}
		type="text"
		role="combobox"
		aria-expanded="true"
		aria-controls="{uid}-list"
		aria-activedescendant={results.length ? `${uid}-option-${index}` : undefined}
		aria-label={label}
		autocomplete="off"
		spellcheck="false"
		{placeholder}
	/>
	<ul id="{uid}-list" role="listbox" aria-label={label}>
		{#each results as command, i (command.id)}
			{#if command.group && command.group !== results[i - 1]?.group}
				<li class="group" role="presentation">{command.group}</li>
			{/if}
			<li
				id="{uid}-option-{i}"
				role="option"
				data-command={command.id}
				aria-selected={i === index}
				class:active={i === index}
				onclick={() => run(command)}
				onmousemove={() => (index = i)}
			>
				<span class="label">{command.label}</span>
				{#if command.hint}<span class="hint">{command.hint}</span>{/if}
			</li>
		{:else}
			<li class="empty" role="presentation">{emptyLabel}</li>
		{/each}
	</ul>
	<p class="keys">{hint}</p>
</dialog>

<style>
	.sc-palette {
		box-sizing: border-box;
		width: min(36rem, calc(100vw - 2rem));
		max-height: min(32rem, calc(100dvh - 4rem));
		margin: 12dvh auto auto;
		padding: 0;
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-none);
		background: var(--paper);
		color: var(--ink);
		font-family: var(--font-mono);
		flex-direction: column;
	}
	.sc-palette[open] {
		display: flex;
	}
	.sc-palette::backdrop {
		background: rgb(0 0 0 / 0.4);
	}
	input {
		box-sizing: border-box;
		width: 100%;
		margin: 0;
		padding: var(--space-075) var(--space-1);
		border: 0;
		border-bottom: var(--border-width-regular) solid var(--ink);
		border-radius: 0;
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		outline: none;
	}
	ul {
		flex: 1;
		margin: 0;
		padding: var(--space-025) 0;
		overflow: auto;
		list-style: none;
	}
	li[role='option'] {
		display: flex;
		justify-content: space-between;
		gap: var(--space-1);
		min-height: var(--control-height);
		box-sizing: border-box;
		align-items: center;
		padding: var(--space-05) var(--space-1);
		cursor: pointer;
	}
	/* selected = inverted (cyan is for "live" states only) */
	li.active {
		background: var(--ink);
		color: var(--paper);
	}
	.group {
		padding: var(--space-05) var(--space-1) var(--space-025);
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	.hint {
		color: var(--text-muted);
		text-align: right;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	li.active .hint {
		color: var(--paper);
	}
	.empty {
		padding: var(--space-1);
		color: var(--text-muted);
	}
	.keys {
		margin: 0;
		padding: var(--space-05) var(--space-1);
		border-top: var(--border-width-regular) solid var(--ink);
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
</style>
