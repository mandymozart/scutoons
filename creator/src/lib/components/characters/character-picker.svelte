<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * "+ Character": search the character registry (name, id) or create a new character inline.
	 * Arrow keys, Enter, Escape. `characters` = registry entries with their name in the page's language.
	 */
	let {
		characters,
		listed = new Set<string>(),
		label = m['creator.characters_add'](),
		startOpen = false,
		testid = 'character-picker',
		onpick,
		oncreate,
		oncancel
	}: {
		characters: { id: string; label: string }[];
		/** already in the list (shown, marked) */
		listed?: Set<string>;
		label?: string;
		startOpen?: boolean;
		testid?: string;
		onpick: (id: string) => void;
		oncreate?: (name: string) => void;
		oncancel?: () => void;
	} = $props();

	// svelte-ignore state_referenced_locally
	let open = $state(startOpen);
	let query = $state('');
	let active = $state(0);
	let input = $state<HTMLInputElement>();
	const uid = $props.id();

	const results = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const score = (c: { id: string; label: string }) => {
			const names = [c.label.toLowerCase(), c.id];
			if (!q) return 5;
			if (names.some((n) => n === q)) return 0;
			if (names.some((n) => n.startsWith(q))) return 1;
			if (names.some((n) => n.split(/[\s-]+/).some((w) => w.startsWith(q)))) return 2;
			if (names.some((n) => n.includes(q))) return 3;
			return 9;
		};
		return characters
			.map((c) => [c, score(c)] as const)
			.filter(([, s]) => s < 9)
			.sort((a, b) => a[1] - b[1] || Number(listed.has(a[0].id)) - Number(listed.has(b[0].id)) || a[0].label.localeCompare(b[0].label))
			.slice(0, 12)
			.map(([c]) => c);
	});
	const exact = $derived(results.some((c) => c.label.toLowerCase() === query.trim().toLowerCase() || c.id === query.trim().toLowerCase()));
	const options = $derived([
		...results.map((c) => ({ kind: 'character' as const, character: c })),
		...(oncreate && query.trim() && !exact ? [{ kind: 'create' as const, character: null }] : [])
	]);

	$effect(() => {
		if (open) queueMicrotask(() => input?.focus());
	});

	function start() {
		open = true;
		query = '';
		active = 0;
	}
	function close() {
		open = false;
		oncancel?.();
	}
	function choose(i: number) {
		const option = options[i];
		if (!option) return;
		if (option.kind === 'create') oncreate?.(query.trim());
		else onpick(option.character.id);
		query = '';
		active = 0;
	}
	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown') {
			event.preventDefault();
			active = Math.min(options.length - 1, active + 1);
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			active = Math.max(0, active - 1);
		} else if (event.key === 'Enter') {
			event.preventDefault();
			choose(active);
		} else if (event.key === 'Escape') {
			event.preventDefault();
			close();
		}
	}
</script>

{#if !open}
	<button type="button" class="ghost add" onclick={start} data-testid="{testid}-open">+ {label}</button>
{:else}
	<div class="picker" data-testid={testid}>
		<input
			bind:this={input}
			type="search"
			role="combobox"
			aria-expanded="true"
			aria-controls="{uid}-list"
			aria-activedescendant="{uid}-{active}"
			placeholder={m['creator.characters_search']()}
			bind:value={query}
			oninput={() => (active = 0)}
			{onkeydown}
			data-testid="{testid}-input"
		/>
		<button type="button" class="ghost" onclick={close} aria-label="Close"><Icon name="x" /></button>
		<ul id="{uid}-list" role="listbox">
			{#each options as option, i (option.character?.id ?? 'create')}
				<li id="{uid}-{i}" role="option" aria-selected={i === active} class:active={i === active}>
					<button type="button" class="option" onmousedown={(e) => e.preventDefault()} onclick={() => choose(i)} data-testid="{testid}-option">
						{#if option.kind === 'create'}
							{m['creator.characters_create_named']({ name: query.trim() })}
						{:else}
							{option.character.label} <span class="hint">· {option.character.id}</span>
							{#if listed.has(option.character.id)}<span class="hint">· {m['creator.characters_in_cast']()}</span>{/if}
						{/if}
					</button>
				</li>
			{:else}
				<li class="hint">{m['creator.characters_none_found']()}</li>
			{/each}
		</ul>
	</div>
{/if}

<style>
	.picker {
		position: relative;
		display: flex;
		gap: 0.25rem;
		align-items: center;
		max-width: 26rem;
	}
	.picker input {
		flex: 1;
		min-width: 0;
	}
	ul {
		position: absolute;
		top: 100%;
		left: 0;
		right: 0;
		z-index: 30;
		list-style: none;
		margin: 0;
		padding: 0;
		background: var(--paper);
		border: 2px solid var(--ink);
		max-height: 18rem;
		overflow: auto;
	}
	li.active {
		background: var(--ink);
		color: var(--paper);
	}
	li.hint {
		padding: 0.3rem 0.5rem;
	}
	.option {
		all: unset;
		display: block;
		width: 100%;
		box-sizing: border-box;
		padding: 0.3rem 0.5rem;
		cursor: pointer;
	}
	.add {
		align-self: flex-start;
	}
</style>
