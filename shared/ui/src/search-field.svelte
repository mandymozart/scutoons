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
		compact = false,
		onsearch
	}: {
		value?: string;
		label: string;
		placeholder?: string;
		delay?: number;
		/** one line: the label becomes the placeholder / accessible name (e.g. inside the ActionBar) */
		compact?: boolean;
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

<div class="sc-search" class:compact>
	<label for="search-{uid}" class:sr-only={compact}>{label}</label>
	<input
		id="search-{uid}"
		type="search"
		autocomplete="off"
		spellcheck="false"
		placeholder={placeholder ?? (compact ? label : undefined)}
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
		font-weight: var(--font-weight-bold);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
	}
	input {
		box-sizing: border-box;
		width: 100%;
		min-height: var(--control-height);
		margin: 0;
		padding: var(--space-025) var(--space-05);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-none);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
	}
	.compact {
		min-width: 10rem;
	}
	.compact input {
		padding: var(--space-025) var(--space-05);
	}
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}
	input:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}
</style>
