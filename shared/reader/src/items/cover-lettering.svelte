<script lang="ts">
	/**
	 * A title lettered onto an item's cover – a score's song title on the shared blank songsheet (agents/research/SCORES.md
	 * §1: lettering is text, never baked into the picture; the score's own language, not the reader's). Laid over its
	 * parent (which must be positioned) and sized from the parent's width (container units), so it scales with the
	 * icon in the inventory, the song book and on the slice.
	 */
	let { text }: { text: string } = $props();
</script>

<span class="cover-lettering" aria-hidden="true"><span class="title">{text}</span></span>

<style>
	.cover-lettering {
		position: absolute;
		inset: 18% 16% 34%;
		container-type: inline-size;
		display: flex;
		align-items: center;
		justify-content: center;
		pointer-events: none;
	}
	.title {
		max-width: 100%;
		font-family: var(--font-lettering, inherit);
		font-size: 22cqi;
		line-height: 1.05;
		text-align: center;
		color: var(--ink, #000);
		overflow-wrap: anywhere;
		display: -webkit-box;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
</style>
