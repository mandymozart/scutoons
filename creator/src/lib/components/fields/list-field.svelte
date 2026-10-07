<script lang="ts">
	import { movedAt } from '$lib/editor/move.ts';
	import { CloseIcon, Icon, SortGrip, sortable } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';

	/** An editable list of names (authors, credits): one input per entry, remove, reorder, add. */
	let {
		value,
		onchange,
		placeholder = 'Name',
		testid = 'list'
	}: { value: unknown; onchange: (list: string[]) => void; placeholder?: string; testid?: string } = $props();

	const list = $derived<string[]>(Array.isArray(value) ? value.map((v) => String(v ?? '')) : typeof value === 'string' && value ? [value] : []);
	let draft = $state('');

	function set(index: number, text: string) {
		const next = [...list];
		next[index] = text;
		onchange(next);
	}
	function remove(index: number) {
		onchange(list.filter((_, i) => i !== index));
	}
	function move(from: number, to: number) {
		const next = movedAt(list, from, to);
		if (next) onchange(next);
	}
	function add() {
		const text = draft.trim();
		if (!text) return;
		onchange([...list, text]);
		draft = '';
	}
</script>

<ul class="list-field" use:sortable={{ onmove: move }} data-testid={testid}>
	{#each list as item, i (i)}
		<li data-sort-row>
			<SortGrip label={m['creator.sort_move']({ name: item || `${placeholder} ${i + 1}` })} title={m['creator.sort_hint']()} />
			<input type="text" value={item} oninput={(e) => set(i, e.currentTarget.value)} aria-label="{placeholder} {i + 1}" />
			<button type="button" class="ghost" onclick={() => remove(i)} title={m['creator.remove']()} aria-label={m['creator.remove']()}><CloseIcon /></button>
		</li>
	{/each}
	<li class="add">
		<input
			type="text"
			bind:value={draft}
			{placeholder}
			onkeydown={(e) => {
				if (e.key === 'Enter') {
					e.preventDefault();
					add();
				}
			}}
			data-testid="{testid}-new"
		/>
		<button type="button" class="secondary" onclick={add} disabled={!draft.trim()} data-testid="{testid}-add">+</button>
	</li>
</ul>

<style>
	.list-field {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}
	li {
		display: flex;
		gap: 0.2rem;
		align-items: center;
	}
	li input {
		flex: 1;
		min-width: 0;
	}
	li button {
		padding: 0 0.4em;
	}
</style>
