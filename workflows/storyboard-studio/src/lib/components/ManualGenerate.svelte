<script>
	// Luma – by hand in the Generate panel (generators/luma-manual.mjs; owner 2026-10-19: "the luma tab is weird. this
	// should be somehow now be covered by the generate right?"): what the Luma panel offered, in Generate's place – the
	// prompt built from the shot (GeneratePanel's Prompt row, read-only), the render without texts (Luma's image
	// reference), the references to attach; the result comes back by dropping it onto the board (its shot's band – a
	// source card, board-sources.svelte.js). One primary (RULES): "Build from the shot" until there is a prompt, then
	// Copy prompt. README "Generators".
	import { fail, notify, studio } from '../studio.svelte.js';
	import { framePng } from '../frames.js';
	import { gen } from '../generate.svelte.js';

	/** the prompt is built for the current shot */
	let { built } = $props();

	async function copy() {
		await navigator.clipboard.writeText(gen.built?.text ?? '');
		notify('Prompt copied – paste it into Luma.', 'success');
	}
	async function downloadRender() {
		const a = document.createElement('a');
		a.href = await framePng({ clean: true });
		a.download = `${studio.sel.id}-structure.png`;
		a.click();
	}
</script>

<!-- right under the prompt (Copy stays in sight above the long list), the primary last -->
<div class="buttons">
	<button type="button" class="secondary" onclick={() => downloadRender().catch(fail)} title="The render as it is now, without texts – Luma's image reference">Render (no texts)</button>
	{#if built}<button type="button" onclick={() => copy().catch(fail)}>Copy prompt</button>{/if}
</div>
{#if built}
	<fieldset>
		<legend>Attach in Luma</legend>
		<ul>
			{#each gen.built?.refs ?? [] as r, i (i)}
				<li>{#if r.url}<a href={r.url} download={r.name ?? ''}>{r.name}</a>{:else}<span class="muted">no image yet</span>{/if} – {r.role}</li>
			{/each}
		</ul>
	</fieldset>
{/if}

<style>
	fieldset {
		margin: 0;
	}
	ul {
		margin: 0;
		padding-left: var(--space-1);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		overflow-wrap: anywhere;
	}
	.muted {
		color: var(--text-muted);
	}
	/* the primary last (RULES); a row too narrow for both wraps, each keeps its words on one line */
	.buttons {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: var(--space-05);
	}
	.buttons button {
		white-space: nowrap;
	}
</style>
