<script>
	// The prompt for Luma (scripts/storyboard/luma.js) from the shot as it is set right now (also unsaved): the story
	// moment, perspective, figures, camera, light, empty zones, the look – copy it; attach the clean render (no texts)
	// and the character / style pictures listed.
	import { api, ch, studio, toon } from '../studio.svelte.js';
	import { framePng } from '../frames.js';

	let text = $state('');
	let refs = $state([]);
	let busy = $state(false);

	async function build() {
		busy = true;
		try {
			const r = await api('POST', '/api/prompt', { toon: toon(), ch: ch(), shot: $state.snapshot(studio.sel.shot) });
			text = r.text;
			refs = r.refs;
		} finally {
			busy = false;
		}
	}
	async function copy() {
		await navigator.clipboard.writeText(text);
		studio.message = 'Prompt copied – paste it into Luma.';
	}
	async function downloadRender() {
		const a = document.createElement('a');
		a.href = await framePng({ clean: true });
		a.download = `${studio.sel.id}-structure.png`;
		a.click();
	}
	$effect(() => { void studio.sel?.id; text = ''; refs = []; });
</script>

{#if studio.sel}
	<div class="panel">
		<p class="hint">Everything set for this shot – also unsaved changes – as one prompt.</p>
		<div class="row">
			<button type="button" class="secondary" onclick={downloadRender} title="The render as it is now, without texts – Luma's image reference">Render (no texts)</button>
			<button type="button" onclick={build} disabled={busy}>{text ? 'Rebuild prompt' : 'Build prompt'}</button>
		</div>
		{#if text}
			<textarea readonly rows="22" value={text}></textarea>
			<div class="row"><button type="button" onclick={copy}>Copy prompt</button></div>
			<h3>Attach in Luma</h3>
			<ul>
				{#each refs as r}
					<li>{#if r.url}<a href={r.url} download={r.name ?? ''}>{r.name}</a>{:else}<span>no image yet</span>{/if} – {r.role}</li>
				{/each}
			</ul>
		{/if}
	</div>
{/if}

<style>
	.panel {
		padding: var(--space-1);
		display: grid;
		gap: var(--space-1);
	}
	.row {
		display: flex;
		gap: var(--space-05);
		justify-content: flex-end;
	}
	textarea {
		width: 100%;
		min-height: 20rem;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		resize: vertical;
	}
	h3 {
		margin: 0;
		font-size: var(--type-caption-size);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	ul {
		margin: 0;
		padding-left: var(--space-1);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.hint {
		margin: 0;
		font-size: var(--type-caption-size);
	}
</style>
