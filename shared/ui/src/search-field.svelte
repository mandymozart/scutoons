<script lang="ts">
	/**
	 * Debounced search input for URL-driven lists: `onsearch(q)` fires `delay` ms after typing stops and on Enter; the
	 * app turns it into a URL (`listHref(url, { q })`). `value` follows the URL when it changes from outside.
	 */
	let {
		value = '',
		label,
		placeholder,
		delay = 300,
		onsearch
	}: {
		value?: string;
		label: string;
		placeholder?: string;
		delay?: number;
		onsearch: (q: string) => void;
	} = $props();

	const uid = $props.id();
	let text = $state('');
	let timer: ReturnType<typeof setTimeout> | undefined;
	let last = '';

	$effect.pre(() => {
		// outside change (back button, link): take it over
		if (value !== last) {
			text = value;
			last = value;
		}
	});

	function fire() {
		clearTimeout(timer);
		const q = text.trim();
		if (q === last) return;
		last = q;
		onsearch(q);
	}

	function input() {
		clearTimeout(timer);
		timer = setTimeout(fire, delay);
	}

	function keydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault();
			fire();
		}
	}
</script>

<div class="sc-search">
	<label for="search-{uid}">{label}</label>
	<input
		id="search-{uid}"
		type="search"
		autocomplete="off"
		spellcheck="false"
		{placeholder}
		bind:value={text}
		oninput={input}
		onkeydown={keydown}
	/>
</div>

<style>
	.sc-search {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 12rem;
	}
	label {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		color: var(--text-muted);
	}
	input {
		box-sizing: border-box;
		width: 100%;
		margin: 0;
		padding: var(--space-05) var(--space-075);
		border: var(--border-width-hairline) solid var(--border-subtle);
		border-radius: var(--radius-none);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
	}
	input:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: -1px;
	}
</style>
