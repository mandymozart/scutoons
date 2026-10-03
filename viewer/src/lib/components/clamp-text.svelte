<script lang="ts">
	/**
	 * A text clamped to `lines` with an ellipsis; when it is longer, a tap / click (Enter / Space) opens and closes it
	 * (owner 2026-10-03: the season description). Short texts stay plain text.
	 */
	let { text, lines = 3, class: className = '' }: { text: string; lines?: number; class?: string } = $props();

	let el = $state<HTMLElement>();
	let open = $state(false);
	let long = $state(false);

	$effect(() => {
		const node = el;
		void text;
		if (!node) return;
		const measure = () => {
			if (!open) long = node.scrollHeight > node.clientHeight + 1;
		};
		measure();
		const observer = new ResizeObserver(measure);
		observer.observe(node);
		return () => observer.disconnect();
	});

	function toggle() {
		if (long) open = !open;
	}
</script>

<!-- a button only when there is more to show -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<p
	bind:this={el}
	class="clamp-text {className}"
	class:clamped={!open}
	class:long
	style:--clamp-lines={lines}
	role={long ? 'button' : undefined}
	tabindex={long ? 0 : undefined}
	aria-expanded={long ? open : undefined}
	onclick={toggle}
	onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && long && (e.preventDefault(), toggle())}
>{text}</p>

<style>
	.clamped {
		display: -webkit-box;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: var(--clamp-lines);
		line-clamp: var(--clamp-lines);
		overflow: hidden;
	}
	.long {
		cursor: pointer;
	}
</style>
