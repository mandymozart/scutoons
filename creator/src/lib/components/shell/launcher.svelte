<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { fade } from 'svelte/transition';
	import { page } from '$app/state';
	import { Icon, clickOutside } from '@scutoons/ui';
	import { ListRow, motionMs } from '@scutoons/ui/editor';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { rank, recentFirst, type LaunchEntry, type LaunchKind } from '$lib/shell/launcher.ts';
	import { assets, commandEntries, items, launch, loadSources, typed, views, workspaces } from '$lib/shell/launch-sources.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';

	/**
	 * The launcher (crafts-v2.html view `launch`; Spotlight / rofi): one field that finds any item (by name or `kind:id`),
	 * workspace, view, file or command, in one ranked list – the recent picks while nothing is typed. ↑ ↓ move, Enter opens,
	 * Esc closes. Over everything, centred (Alt Space, Cmd Space, Ctrl K), or `inline` as the dashboard's top field.
	 */
	let { inline = false, onpick }: { inline?: boolean; onpick?: () => void } = $props();

	const nav = getCreatorNav();
	const session = getCreatorSession();
	const logout = () => session.logout();
	let query = $state('');
	let index = $state(0);
	let open = $state(false);
	let input = $state<HTMLInputElement>();
	const uid = $props.id();

	$effect(() => {
		if (open) loadSources(nav);
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

	function close() {
		query = '';
		if (inline) open = false;
		else shell.toggleLauncher(false);
	}
	function pick(entry: LaunchEntry | undefined) {
		if (!entry) return;
		shell.remember(entry);
		close();
		onpick?.();
		void tick().then(() => launch(entry, logout));
	}

	function onkeydown(event: KeyboardEvent) {
		const count = results.length;
		if (event.key === 'ArrowDown' || (event.key === 'Tab' && !inline && !event.shiftKey)) {
			event.preventDefault();
			open = true;
			index = count ? (index + 1) % count : 0;
		} else if (event.key === 'ArrowUp' || (event.key === 'Tab' && !inline && event.shiftKey)) {
			event.preventDefault();
			index = count ? (index - 1 + count) % count : 0;
		} else if (event.key === 'Enter') {
			event.preventDefault();
			pick(results[index]);
		} else if (event.key === 'Escape' && !inline) {
			event.preventDefault();
			event.stopPropagation();
			close();
		}
	}

	// over everything: the focus goes back where it was when the launcher closes
	onMount(() => {
		if (inline) return;
		open = true;
		const was = document.activeElement as HTMLElement | null;
		input?.focus();
		return () => was?.focus?.();
	});
</script>

{#snippet field()}
	<form class="find" role="search" onsubmit={(e) => e.preventDefault()}>
		<Icon name="search" size="1rem" />
		<input
			bind:this={input}
			type="search"
			bind:value={query}
			onfocus={() => (open = true)}
			oninput={() => (open = true)}
			{onkeydown}
			placeholder={m['shell.find_placeholder']()}
			aria-label={m['shell.find']()}
			role="combobox"
			aria-expanded={open && results.length > 0}
			aria-controls="{uid}-list"
			aria-activedescendant={open && results.length ? `${uid}-${index}` : undefined}
			autocomplete="off"
			spellcheck="false"
			data-testid="launcher-input"
		/>
	</form>
{/snippet}

{#snippet list()}
	<div class="rows" id="{uid}-list" role="listbox" aria-label={m['shell.find']()} data-testid="launcher-list">
		{#if !query.trim() && results.length}<p class="head">{shell.picks.length ? m['shell.launch_recent']() : m['shell.workspaces']()}</p>{/if}
		{#each results as entry, i (entry.id)}
			<ListRow id="{uid}-{i}" role="option" aria-selected={i === index} icon={entry.icon} label={entry.label} detail={entry.hint !== KIND_NAMES[entry.kind]() && entry.hint !== entry.label ? entry.hint : undefined} meta={KIND_NAMES[entry.kind]()} title={entry.address ?? entry.label} selected={i === index} onclick={() => pick(entry)} data-testid="launcher-row" data-kind={entry.kind} />
		{:else}
			{#if query.trim()}<p class="sc-empty">{m['shell.launch_none']()}</p>{/if}
		{/each}
	</div>
	<p class="keys">{m['shell.launch_keys']()}</p>
{/snippet}

{#if inline}
	<div class="launcher inline" {@attach clickOutside(() => (open = false))} data-testid="launcher-inline">
		{@render field()}
		{#if open && results.length}<div class="drop">{@render list()}</div>{/if}
	</div>
{:else}
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div class="scrim" onclick={(e) => e.target === e.currentTarget && close()} data-testid="launcher" out:fade={{ duration: motionMs('fast') }}>
		<div class="launcher over" role="dialog" aria-modal="true" aria-label={m['shell.launch_label']()}>
			{@render field()}
			{@render list()}
		</div>
	</div>
{/if}

<style>
	.launcher {
		position: relative;
		width: 100%;
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
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: 1.2rem;
		background: var(--tool-panel);
		text-shadow: var(--tool-halo);
		color: var(--tool-muted);
	}
	/* the halo on its icons too (the deck's ink ring covers text and icons) */
	.find :global(svg) {
		filter: var(--tool-icon-halo);
	}
	.find input {
		flex: 1;
		min-width: 0;
		min-height: calc(var(--control-height) - 0.2rem);
		border: 0;
		background: transparent;
		color: var(--tool-text);
		outline: none;
	}
	.drop {
		position: absolute;
		z-index: 60;
		top: calc(100% + 0.45rem);
		left: 0;
		right: 0;
		padding: 0.3rem;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: var(--editor-radius);
		background: var(--tool-popover);
	}
	.rows {
		display: flex;
		flex-direction: column;
		max-height: min(28rem, 60vh);
		overflow: auto;
	}
	.head {
		margin: 0.3rem 0 0.2rem;
		padding: 0 calc(var(--editor-pad) / 2);
		color: var(--tool-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.keys {
		margin: 0.3rem 0 0;
		padding: 0 calc(var(--editor-pad) / 2);
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
	/* over everything: the stage veiled, the field and its list centred a third down */
	.scrim {
		position: fixed;
		z-index: 70;
		inset: 0;
		display: flex;
		justify-content: center;
		align-items: flex-start;
		padding-top: 18vh;
		background: var(--tool-scrim);
		backdrop-filter: blur(0.4rem);
		animation: launch-in var(--motion-fast) var(--motion-ease);
	}
	:global([data-theme='day']) .scrim {
		background: color-mix(in srgb, var(--tool-ground) 60%, transparent);
	}
	.over {
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
		box-sizing: border-box;
		width: min(40rem, 100% - 2rem);
		padding: 0.45rem;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: calc(var(--editor-radius) + 0.45rem);
		background: var(--tool-popover);
	}
	@keyframes launch-in {
		from {
			opacity: 0;
		}
	}
</style>
