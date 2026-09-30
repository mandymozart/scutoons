<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';
	// Short rules from agents/DESIGN.md (Part 2 "Text box on slice") and the PLAN.md decisions.
	let { onclose }: { onclose?: () => void } = $props();
</script>

<div class="style-guide" data-testid="style-guide">
	<header>
		<h2>SCUTOONS {m['creator.style_guide']()}</h2>
		{#if onclose}<button class="ghost" onclick={onclose} aria-label="Close">✕</button>{/if}
	</header>

	<h3>Slices</h3>
	<ul>
		<li>Standard slice: <b>390 × 625 px</b> (the webtoon standard, Naver), delivered at <b>1x only</b> as <b>WebP</b>. Other heights work, the width stays 390.</li>
		<li>Images live on the CDN under <code>&lt;toon&gt;/images/webtoon/s1c2/s1c2-slice-N.webp</code> (one folder per chapter; <code>cdn/tools/make_slice_webp.py</code> turns delivered JPGs into these).</li>
		<li>Every slice carries its <code>width</code> / <code>height</code> – the editor reads them from the image when you set its path (a missing size is an error).</li>
		<li>A slice can be a <b>video</b> (muted, looping, plays in view): H.264 MP4 ≤ 3 MB, the slice image is its poster.</li>
		<li>One reading column, max 500 px wide – all positions are <b>% of the slice</b>, font sizes <b>% of the slice width</b>.</li>
	</ul>

	<h3>Text-free art</h3>
	<ul>
		<li>Export the art <b>without words</b>. Bubble <i>shapes</i> stay in the art; the words are live text boxes.</li>
		<li>Every locale shares the same images – only the text changes (the toon's source language is the source, the other languages follow; set it on the toon page under Languages).</li>
		<li>Legacy art with baked text (chapter 1): cover it with <code>mask</code> (+ <code>pad</code>, <code>maskShape</code>).</li>
		<li>Leave room: translations often run ~10–30% longer (German → English). Boxes over 130% of the source length get a warning.</li>
	</ul>

	<h3>Lettering</h3>
	<ul>
		<li><b>lettering</b> = Bangers (bubbles, SFX, titles) · <b>mono</b> = IBM Plex Mono (captions, narration).</li>
		<li>Default size 4 (% of slice width ≈ 15.6 px at 390). Chapter 1 captions ≈ 3.8–4.1.</li>
		<li>Ink on paper; accent colours only for SFX. No shadows, square boxes.</li>
	</ul>

	<h3>Which variant?</h3>
	<dl>
		<dt>bubble</dt>
		<dd>Someone speaks or thinks – the text sits inside a bubble drawn in the art. Centered, lettering.</dd>
		<dt>caption</dt>
		<dd>Place, time, a sign, a character's inner voice in a box: paper box, 2 px ink border, mono, left.</dd>
		<dt>narration</dt>
		<dd>The narrator's voice (screenplay prose): like caption, but italic. <code>frame: false</code> for bare text.</dd>
		<dt>sfx</dt>
		<dd>Sounds (GRRRR, ZISCH): outlined lettering, uppercase, may rotate. Short, loud, one or two words.</dd>
	</dl>
</div>

<style>
	.style-guide {
		font-size: 12px;
		line-height: 1.45;
	}
	header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		border-bottom: 2px solid var(--ink);
		margin-bottom: 0.5rem;
	}
	h2 {
		font-size: 13px;
		margin: 0.25rem 0;
	}
	h3 {
		font-size: 12px;
		margin: 0.9rem 0 0.25rem;
		text-transform: uppercase;
	}
	ul {
		margin: 0;
		padding-left: 1.1rem;
	}
	li {
		margin-bottom: 0.25rem;
	}
	dl {
		margin: 0;
	}
	dt {
		font-weight: 700;
		margin-top: 0.4rem;
	}
	dd {
		margin: 0 0 0 0.75rem;
	}
	code {
		background: var(--surface-panel);
		padding: 0 0.2em;
	}
</style>
