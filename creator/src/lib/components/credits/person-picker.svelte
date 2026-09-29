<script lang="ts">
	import type { CreditPerson } from '@scutoons/content';
	import { searchPeople } from '$lib/editor/credits.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** "+ Person": search the registry (name, aliases) or create a new person inline. Arrow keys, Enter, Escape. */
	let {
		people,
		listed = new Set<string>(),
		label = m['creator.credits_add_person'](),
		testid = 'person-picker',
		onpick,
		oncreate
	}: {
		people: CreditPerson[];
		/** already in this list (shown, marked) */
		listed?: Set<string>;
		label?: string;
		testid?: string;
		onpick: (id: string) => void;
		oncreate: (name: string) => void;
	} = $props();

	let open = $state(false);
	let query = $state('');
	let active = $state(0);
	let input = $state<HTMLInputElement>();
	const uid = $props.id();

	const results = $derived(searchPeople(people, query, 10));
	const exact = $derived(results.some((p) => p.name.toLowerCase() === query.trim().toLowerCase()));
	const options = $derived([
		...results.map((p) => ({ kind: 'person' as const, person: p })),
		...(query.trim() && !exact ? [{ kind: 'create' as const, person: null }] : [])
	]);

	function start() {
		open = true;
		query = '';
		active = 0;
		queueMicrotask(() => input?.focus());
	}
	function choose(i: number) {
		const option = options[i];
		if (!option) return;
		if (option.kind === 'create') oncreate(query.trim());
		else onpick(option.person.id);
		query = '';
		active = 0;
		input?.focus();
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
			open = false;
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
			placeholder={m['creator.credits_search_people']()}
			bind:value={query}
			oninput={() => (active = 0)}
			{onkeydown}
			data-testid="{testid}-input"
		/>
		<button type="button" class="ghost" onclick={() => (open = false)} aria-label="Close">✕</button>
		<ul id="{uid}-list" role="listbox">
			{#each options as option, i (option.person?.id ?? 'create')}
				<li id="{uid}-{i}" role="option" aria-selected={i === active} class:active={i === active}>
					<button type="button" class="option" onmousedown={(e) => e.preventDefault()} onclick={() => choose(i)} data-testid="{testid}-option">
						{#if option.kind === 'create'}
							{m['creator.credits_create_person']({ name: query.trim() })}
						{:else}
							{option.person.name}
							{#if option.person.aliases?.length}<span class="hint">({option.person.aliases.join(', ')})</span>{/if}
							{#if listed.has(option.person.id)}<span class="hint">· {m['creator.credits_listed']()}</span>{/if}
						{/if}
					</button>
				</li>
			{:else}
				<li class="hint">{m['creator.credits_type_a_name']()}</li>
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
		background: var(--surface-hover);
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
		font-size: 13px;
	}
	.add {
		align-self: flex-start;
	}
</style>
