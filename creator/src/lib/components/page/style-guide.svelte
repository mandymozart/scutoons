<script lang="ts">
	import { DEFAULT_SLICE } from '@scutoons/content';
	// The SCUTOONS style guide for creators: agents/DESIGN.md (Part 2 "Text box on slice"), the PLAN.md rules (slice size
	// 390 × 625, 1x only) and the content model in @scutoons/content (slices, characters, credits). Shown in full on
	// /guide, the chapter editor's Info panel shows the same sections.
	let { sections = ['slices', 'art', 'lettering', 'variants', 'characters', 'credits'] }: { sections?: string[] } = $props();
	const W = DEFAULT_SLICE.width;
	const H = DEFAULT_SLICE.height;
</script>

<div class="style-guide" data-testid="style-guide">
	{#if sections.includes('slices')}
		<h3>Slices</h3>
		<ul>
			<li>One size in the whole app: the standard slice is {W} × {H} px (the webtoon standard, Naver), delivered and shown at 1x only, as WebP. There are no 2x / 3x exports.</li>
			<li>Other heights are fine, the width stays {W}. The editor warns when an image is not {W} px wide.</li>
			<li>Path on the CDN: <code>&lt;toon&gt;/s&lt;n&gt;/c&lt;n&gt;/&lt;sXcY&gt;-slice-N.webp</code>, the chapter's folder (e.g. <code>s1/c2/s1c2-slice-1.webp</code>); density variants as suffixes (<code>.2x</code>, <code>.3x</code>).</li>
			<li>Every slice carries <code>width</code> / <code>height</code> (required – a missing size is an error). The editor reads them from the image when you set its path; new slices start at {W} × {H}.</li>
			<li>Video slice (optional): <code>video.src</code> = an H.264 MP4 (≤ 3 MB, a new file name for every new version), muted, inline, looping while the slice is in view. The slice image is its poster (still required, with its size); with reduced motion only the poster shows. <code>loop</code> / <code>autoplay</code> default to on.</li>
			<li>All positions are % of the slice, font sizes % of the slice width – they scale with the reading column (max 500 px).</li>
		</ul>
	{/if}

	{#if sections.includes('art')}
		<h3>Text-free art</h3>
		<ul>
			<li>Export the art without words. Bubble <i>shapes</i> stay in the art; the words are live text boxes.</li>
			<li>Every language shares the same images – only the text changes. The toon's source language (toon page → Languages) is the source, the others are translations.</li>
			<li><code>mask</code> (+ <code>pad</code>, <code>maskShape</code>) is deprecated: art with baked text (chapter 1) is re-imported text-free; the text boxes stay.</li>
			<li>Leave room: translations often run ~10–30 % longer (German → English). Boxes over 130 % of the source length get a warning.</li>
		</ul>
	{/if}

	{#if sections.includes('lettering')}
		<h3>Lettering</h3>
		<ul>
			<li>lettering = Bangers (bubbles, onomatopoeia, titles) · mono = IBM Plex Mono (captions, narration).</li>
			<li>Default size 4 (% of the slice width ≈ {Math.round(W * 0.04 * 10) / 10} px at {W}). Chapter 1 captions ≈ 3.8–4.1.</li>
			<li>Ink on paper; accent colours only for onomatopoeia. No shadows, square boxes.</li>
		</ul>
	{/if}

	{#if sections.includes('variants')}
		<h3>Which variant?</h3>
		<dl>
			<dt>bubble</dt>
			<dd>Someone speaks or thinks – centered lettering, in a bubble drawn in the art or drawn by the box (<code>frame: true</code>, a tail).</dd>
			<dt>caption</dt>
			<dd>Place, time, a sign, a character's inner voice in a box: paper box, 2 px ink border, mono, left.</dd>
			<dt>narration</dt>
			<dd>The narrator's voice (screenplay prose): like caption, but italic. <code>frame: false</code> for bare text.</dd>
			<dt>onomatopoeia</dt>
			<dd>Sounds (GRRRR, ZISCH): a bubble without frame – lettering, uppercase, white with an ink outline, may rotate and scale – revealed by an onomatopoeia event with its effect. Short, loud, one or two words.</dd>
		</dl>
	{/if}

	{#if sections.includes('characters')}
		<h3>Characters</h3>
		<ul>
			<li>Character → persona (another identity: human, cat, biker; every character has <code>regular</code>) → look (outfit right now, could come off in the next panel; one default per persona) → mood (expression, open vocabulary; only <code>neutral</code> is required) × view.</li>
			<li>Only the portrait is drawn. The avatar (192 px) is generated from it; <code>full</code> is not used.</li>
			<li>Deliver PNG under the convention name; the characters import writes the WebP (q85) and the avatar: <code>characters/&lt;character&gt;[-&lt;persona&gt;]/&lt;character&gt;[-&lt;persona&gt;][_&lt;look&gt;]_&lt;view&gt;-&lt;mood&gt;.webp</code>.</li>
			<li>Transitions are named after their looks: <code>…_transition-&lt;from&gt;-to-&lt;to&gt;</code> (e.g. <code>yellowmouse_transition-helmet-to-default</code>).</li>
			<li>A-pose (the members' avatar body, per character): full body, neutral, 768 × 1152, <code>characters/&lt;character&gt;/&lt;character&gt;_apose-&lt;look&gt;.webp</code>.</li>
			<li>A missing image falls back step by step (same look neutral → default look → regular persona → the single image → a “missing” placeholder) – scenes keep working. Wanted images are listed on the Characters page.</li>
		</ul>
	{/if}

	{#if sections.includes('credits')}
		<h3>Credits</h3>
		<ul>
			<li>People are one registry for every toon: add a person once, use them everywhere; merge duplicates (the old name becomes an alias).</li>
			<li>Roles come from the shared vocabulary; a toon can add its own roles. A role can carry a detail and a language (translations).</li>
			<li>Sections: the toon (its lead = “Created by”, shown first and prominently), season, chapter, song, performance and thanks. The reader's credits page shows them in the reader's language.</li>
		</ul>
	{/if}
</div>

<style>
	.style-guide {
		line-height: 1.5;
	}
	h3 {
		margin: 1rem 0 0.25rem;
		text-transform: uppercase;
	}
	h3:first-child {
		margin-top: 0;
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
		font-weight: var(--font-weight-regular);
		margin-top: 0.4rem;
	}
	dd {
		margin: 0 0 0 0.75rem;
	}
	code {
		background: var(--surface-panel);
		padding: 0 0.2em;
		overflow-wrap: anywhere;
	}
</style>
