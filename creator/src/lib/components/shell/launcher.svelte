<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { page } from '$app/state';
	import { Icon } from '@scutoons/ui';
	import { ListRow, Overlay } from '@scutoons/ui/editor';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { rank, recentFirst, type LaunchEntry, type LaunchKind } from '$lib/shell/launcher.ts';
	import { assets, commandEntries, items, launch, loadSources, typed, views, workspaces } from '$lib/shell/launch-sources.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';

	/**
	 * Search (lap2 A4; Spotlight / rofi): its own overlay (Alt Space, Cmd Space) – one field that finds any item (by name or
	 * `kind:id`), workspace, view, file or command, in one ranked list, each line with what it is; the recent picks while
	 * nothing is typed. ↑ ↓ move, Enter opens, Esc or a click outside closes. The keys are the field's hover text.
	 */
	const nav = getCreatorNav();
	const session = getCreatorSession();
	const logout = () => session.logout();
	let query = $state(shell.searchSeed);
	let index = $state(0);
	let input = $state<HTMLInputElement>();
	const uid = $props.id();

	onMount(() => {
		loadSources(nav);
		input?.focus();
		input?.setSelectionRange(query.length, query.length);
	});

	const toon = $derived((page.params.toonId as string | undefined) ?? null);
	const listed = $derived([...items(nav, toon), ...workspaces(), ...views(), ...commandEntries(logout)]);
	const results = $derived.by(() => {
		const q = query.trim();
		if (!q) return recentFirst(listed, shell.picks, workspaces());
		return rank([...listed, ...typed(q), ...assets(q)], q, shell.picks.map((p) => p.id));
	});
	$effect(() => {
		void query;
		index = 0;
	});
	$effect(() => {
		document.getElementById(`${uid}-${index}`)?.scrollIntoView({ block: 'nearest' });
	});

	const KIND_NAMES: Record<LaunchKind, () => string> = {
		item: () => m['shell.launch_kind_item'](),
		workspace: () => m['shell.launch_workspace'](),
		view: () => m['shell.launch_view'](),
		asset: () => m['shell.launch_kind_asset'](),
		command: () => m['shell.launch_command']()
	};

	const close = () => shell.toggleLauncher(false);
	function pick(entry: LaunchEntry | undefined) {
		if (!entry) return;
		shell.remember(entry);
		close();
		void tick().then(() => launch(entry, logout));
	}

	function onkeydown(event: KeyboardEvent) {
		const count = results.length;
		if (event.key === 'ArrowDown' || (event.key === 'Tab' && !event.shiftKey)) {
			event.preventDefault();
			index = count ? (index + 1) % count : 0;
		} else if (event.key === 'ArrowUp' || (event.key === 'Tab' && event.shiftKey)) {
			event.preventDefault();
			index = count ? (index - 1 + count) % count : 0;
		} else if (event.key === 'Enter') {
			event.preventDefault();
			pick(results[index]);
		} else if (event.key === 'Escape') {
			event.preventDefault();
			event.stopPropagation();
			close();
		}
	}
</script>

<Overlay place="high" label={m['shell.launch_label']()} onclose={close} data-testid="launcher">
	<div class="launcher">
		<form class="find" role="search" onsubmit={(e) => e.preventDefault()}>
			<Icon name="search" size="1rem" />
			<input
				bind:this={input}
				type="search"
				bind:value={query}
				{onkeydown}
				placeholder={m['shell.find_placeholder']()}
				title={m['shell.launch_keys']()}
				aria-label={m['shell.find']()}
				role="combobox"
				aria-expanded={results.length > 0}
				aria-controls="{uid}-list"
				aria-activedescendant={results.length ? `${uid}-${index}` : undefined}
				autocomplete="off"
				spellcheck="false"
				data-testid="launcher-input"
			/>
		</form>
		<div class="rows" id="{uid}-list" role="listbox" aria-label={m['shell.find']()} data-testid="launcher-list">
			{#if !query.trim() && results.length}<p class="head">{shell.picks.length ? m['shell.launch_recent']() : m['shell.workspaces']()}</p>{/if}
			{#each results as entry, i (entry.id)}
				<ListRow id="{uid}-{i}" role="option" aria-selected={i === index} icon={entry.icon} label={entry.label} meta={entry.type ?? KIND_NAMES[entry.kind]()} title={entry.address ?? entry.hint ?? entry.label} selected={i === index} onclick={() => pick(entry)} data-testid="launcher-row" data-kind={entry.kind} />
			{:else}
				{#if query.trim()}<p class="sc-empty">{m['shell.launch_none']()}</p>{/if}
			{/each}
		</div>
	</div>
</Overlay>

<style>
	/* a box a third down the window: the field and its list (the kit's menu ground and line) */
	.launcher {
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
		box-sizing: border-box;
		width: min(40rem, 100vw - 2rem);
		padding: 0.45rem;
		border: var(--editor-edge) solid var(--kit-overlay-box-border);
		border-radius: calc(var(--editor-radius) + 0.45rem);
		background: var(--kit-overlay-box-ground);
		text-shadow: var(--tool-halo);
		color: var(--tool-text);
		font-family: var(--editor-sans);
	}
	.find {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 0.3rem 0 0.9rem;
		border: var(--editor-edge) solid var(--kit-field-rest-border);
		border-radius: 1.2rem;
		background: var(--kit-field-rest-ground);
		color: var(--kit-field-rest-mark);
	}
	.find:focus-within {
		border-color: var(--kit-field-on-border);
		color: var(--kit-field-on-mark);
	}
	.find input {
		flex: 1;
		min-width: 0;
		min-height: calc(var(--control-height) - 0.2rem);
		border: 0;
		background: transparent;
		color: var(--kit-field-rest-text);
		outline: none;
	}
	/* no browser cross in the field: Esc clears nothing, it closes Search */
	.find input::-webkit-search-cancel-button {
		appearance: none;
	}
	.rows {
		display: flex;
		flex-direction: column;
		max-height: min(28rem, 60vh);
		overflow: auto;
	}
	.rows:empty {
		display: none;
	}
	.head {
		margin: 0.3rem 0 0.2rem;
		padding: 0 calc(var(--editor-pad) / 2);
		color: var(--tool-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
</style>
