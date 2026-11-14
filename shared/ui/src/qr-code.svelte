<script lang="ts">
	// A QR code of `text` (qrcode-generator, error correction M) as a scalable SVG on paper: the creator's handheld camera
	// pairing (#469) and the conductor's linked reader (#529) draw it the same way. Nothing without a text.
	import qrcode from 'qrcode-generator';

	let { text, label, testid }: { text: string; label: string; testid?: string } = $props();

	const svg = $derived.by(() => {
		if (!text) return '';
		const q = qrcode(0, 'M');
		q.addData(text);
		q.make();
		return q.createSvgTag({ cellSize: 4, margin: 2, scalable: true });
	});
</script>

{#if svg}
	<div class="qr" role="img" aria-label={label} data-testid={testid}>{@html svg}</div>
{/if}

<style>
	.qr {
		flex: none;
		width: 9rem;
		background: var(--paper);
		line-height: 0;
	}
	.qr :global(svg) {
		display: block;
		width: 100%;
		height: auto;
	}
</style>
