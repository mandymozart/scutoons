<script lang="ts">
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
	function move(index: number, by: number) {
		const to = index + by;
		if (to < 0 || to >= list.length) return;
		const next = [...list];
		const [item] = next.splice(index, 1);
		next.splice(to, 0, item);
		onchange(next);
	}
	function add() {
		const text = draft.trim();
		if (!text) return;
		onchange([...list, text]);
		draft = '';
	}
</script>

<ul class="list-field" data-testid={testid}>
	{#each list as item, i (i)}
		<li>
			<input type="text" value={item} oninput={(e) => set(i, e.currentTarget.value)} aria-label="{placeholder} {i + 1}" />
			<button type="button" class="ghost" onclick={() => move(i, -1)} disabled={i === 0} title="up">↑</button>
			<button type="button" class="ghost" onclick={() => remove(i)} title={m['creator.remove']()}>✕</button>
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
