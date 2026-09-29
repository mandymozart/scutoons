<script lang="ts">
	/** Shows QR SVG markup (made locally by qrcode-svg, never user HTML) with the encoded URL below. */
	let { svg, url, label }: { svg: string; url: string; label: string } = $props();
</script>

<figure class="qr-preview">
	<div class="image" role="img" aria-label={label}>
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		{@html svg}
	</div>
	<figcaption>{url}</figcaption>
</figure>

<style>
	.qr-preview {
		margin: 0;
		display: inline-flex;
		flex-direction: column;
		gap: var(--space-05);
		max-width: 100%;
	}
	.image {
		width: 12rem;
		max-width: 100%;
		border: var(--border-width-regular) solid var(--ink);
		line-height: 0;
	}
	.image :global(svg) {
		width: 100%;
		height: auto;
	}
	figcaption {
		font-size: var(--type-caption-size);
		overflow-wrap: anywhere;
	}
</style>
