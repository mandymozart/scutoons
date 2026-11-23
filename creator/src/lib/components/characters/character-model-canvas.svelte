<script lang="ts">
	import { untrack } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import type { CharacterModel } from '$lib/editor/character-model.svelte.ts';
	import { mountModelPlace, type ModelPlace } from '$lib/editor/model-place.ts';
	import { dropAsset, isAssetDrag, libraryHost, misfitWords, overAsset } from '$lib/editor/library-drop.ts';
	import { library } from '$lib/editor/library.svelte.ts';
	import { toast } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The 3D place (research/redesign/characters.html ?view=3d): the look's model full-bleed on a floor, orbit to look
	 * around, its skeleton and joints over it, its spots, a strip of its clips that plays them and the core's quiet words
	 * for what it lacks; attached to the look's 3D slot, replaced by a drop, an upload or a Library pick. Without a model:
	 * the way in. What the file is comes from the core (CharacterModel); this draws it.
	 */
	let {
		model,
		where,
		attached,
		room,
		onattach
	}: {
		model: CharacterModel;
		/** "Regular › Cassock" */
		where: string;
		/** where the shown model sits: the look's own slot, the persona's, or none yet */
		attached: 'look' | 'persona' | null;
		/** what the panels cover at each side (px): the model stands in the middle of the rest */
		room: { left: number; right: number };
		/** a model file for the look's 3D slot */
		onattach: (path: string) => void;
	} = $props();

	const FIT = { kinds: ['model' as const] };
	let canvas = $state<HTMLCanvasElement>();
	let place: ModelPlace | null = null;
	let input = $state<HTMLInputElement>();
	let over = $state<{ ok: boolean; why?: string } | null>(null);
	let busy = $state(false);
	const facts = $derived(model.facts);
	const file = $derived(model.path.split('/').at(-1) ?? '');
	const clip = $derived(facts && model.clip !== null ? facts.clips[model.clip] : null);
	const count = (n: number) => new Intl.NumberFormat(undefined, { notation: 'compact', maximumFractionDigits: 1 }).format(n);

	function colours() {
		const css = getComputedStyle(document.documentElement);
		const v = (name: string, fallback: string) => css.getPropertyValue(name).trim() || fallback;
		return { bone: v('--tool-text', '#f3f3f0'), spot: v('--tool-pressed', '#f3f3f0'), grid: v('--tool-line', '#3a4034') };
	}

	$effect(() => {
		const el = canvas;
		const bytes = model.bytes;
		const spots = facts?.spots ?? [];
		if (!el || !bytes) return;
		let gone = false;
		let mounted: ModelPlace | null = null;
		void mountModelPlace(el, bytes, spots, colours(), (t) => (model.time = t), untrack(() => ({ left: room.left, right: room.right })))
			.then((p) => {
				if (gone) return p.dispose();
				mounted = place = p;
				p.skeleton(untrack(() => model.skeleton));
				p.room(untrack(() => room.left), untrack(() => room.right));
				p.clip(untrack(() => model.clip), untrack(() => model.playing), untrack(() => model.time));
			})
			.catch((e) => (model.why = e instanceof Error ? e.message : String(e)));
		return () => {
			gone = true;
			mounted?.dispose();
			if (place === mounted) place = null;
		};
	});
	$effect(() => place?.skeleton(model.skeleton));
	$effect(() => place?.room(room.left, room.right));
	$effect(() => {
		const c = model.clip;
		const playing = model.playing;
		place?.clip(c, playing, untrack(() => model.time));
	});

	async function upload(f: File) {
		busy = true;
		try {
			const { path } = await library.upload(f);
			const answer = library.takes(path, FIT);
			if (answer.ok) onattach(path);
			else toast(misfitWords(answer), { type: 'warning', kind: 'library' });
		} catch (e) {
			toast(m['character.place_upload_failed']({ reason: e instanceof Error ? e.message : String(e) }), { type: 'error', kind: 'library' });
		} finally {
			busy = false;
		}
	}
	function pick() {
		libraryHost.pick({ label: `${where} · ${m['character.place_slot']()}`, kinds: FIT.kinds, value: model.path, place: '', onpick: (path) => onattach(path) });
	}
	const hasFiles = (e: DragEvent) => !!e.dataTransfer?.types.includes('Files');
	function dragover(e: DragEvent) {
		if (isAssetDrag(e)) {
			e.preventDefault();
			over = overAsset(e, FIT);
		} else if (hasFiles(e)) {
			e.preventDefault();
			over = { ok: true };
		}
	}
	function drop(e: DragEvent) {
		over = null;
		if (isAssetDrag(e)) dropAsset(e, FIT, (path) => onattach(path));
		else if (hasFiles(e)) {
			e.preventDefault();
			const f = e.dataTransfer?.files[0];
			if (f) void upload(f);
		}
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="place" class:over={!!over} class:misfit={over && !over.ok} ondragover={dragover} ondragleave={() => (over = null)} ondrop={drop} data-testid="character-3d">
	<input bind:this={input} type="file" accept=".glb,model/gltf-binary" hidden onchange={(e) => { const f = e.currentTarget.files?.[0]; if (f) void upload(f); e.currentTarget.value = ''; }} />
	{#if !model.path}
		<span class="tag">{m['character.place_no_model']({ where })}</span>
		<div class="way-in" data-testid="character-3d-drop">
			<Icon name="box" size="2.4rem" strokeWidth={1.5} />
			<span class="big">{over && !over.ok ? over.why : m['character.place_drop']()}</span>
			<span class="row">
				<button type="button" disabled={busy} onclick={() => input?.click()} data-testid="character-3d-upload">{m['character.place_upload']()}</button>
				<button type="button" onclick={pick} data-testid="character-3d-pick">{m['character.place_pick']()}</button>
			</span>
		</div>
	{:else}
		<canvas bind:this={canvas} data-testid="character-3d-canvas"></canvas>
		<span class="tag" data-testid="character-3d-tag">{file}{#if facts} · {m['character.place_counts']({ tris: count(facts.triangles), bones: facts.bones.length })}{/if}</span>
		<div class="attach pill">
			<span class="lab">{m['character.place_attached']()}</span>
			<span class="on">{where}{#if attached === 'persona'} · {m['character.place_from_persona']()}{/if}</span>
			<span class="sep" aria-hidden="true"></span>
			<button type="button" disabled={busy} onclick={() => input?.click()} data-testid="character-3d-replace">{m['character.place_replace']()}</button>
			<button type="button" onclick={pick}>{m['character.place_pick_short']()}</button>
			<button type="button" class:on={model.skeleton} aria-pressed={model.skeleton} onclick={() => (model.skeleton = !model.skeleton)} title={m['character.place_skeleton_keys']()} data-testid="character-3d-skeleton">{m['character.place_skeleton']()}</button>
		</div>
		{#if model.loading}
			<p class="quiet">{m['character.place_reading']()}</p>
		{:else if model.why}
			<p class="quiet warn" data-testid="character-3d-why">{model.why}</p>
		{:else if facts?.missing.length}
			<p class="quiet" data-testid="character-3d-missing">{#each facts.missing as line, i (i)}{#if i}<br />{/if}{line}{/each}</p>
		{/if}
		{#if facts?.clips.length}
			<div class="clips pill" data-testid="character-3d-clips">
				<button type="button" class="round" onclick={() => (model.clip === null ? model.play(0) : (model.playing = !model.playing))} aria-label={model.playing ? m['character.place_pause']() : m['character.place_play']()} title={model.playing ? m['character.place_pause']() : m['character.place_play']()}>
					<Icon name={model.playing ? 'pause' : 'play'} size="0.9rem" />
				</button>
				{#each facts.clips as c, i (i)}
					<button type="button" class:on={model.clip === i} onclick={() => model.play(i)} data-testid="character-3d-clip">{c.name || i + 1} · {c.seconds.toFixed(1)} s</button>
				{/each}
				{#if clip}
					<span class="bar" aria-hidden="true"><i style:width="{Math.min(100, ((model.time % (clip.seconds || 1)) / (clip.seconds || 1)) * 100)}%"></i></span>
					<span class="lab">{(model.time % (clip.seconds || 1)).toFixed(1)} s</span>
					<button type="button" class="quiet-button" onclick={() => model.rest()}>{m['character.place_rest']()}</button>
				{/if}
			</div>
		{/if}
	{/if}
</div>

<style>
	/* the place fills its box; its words and pills keep to the room the panels leave (--in-l / --in-r) */
	.place {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: var(--tool-ground);
		color: var(--tool-text);
		font-family: var(--editor-sans);
	}
	.place.over {
		outline: var(--editor-edge) dashed var(--tool-text);
		outline-offset: -0.5rem;
	}
	.place.misfit {
		outline-color: var(--tool-warn);
	}
	canvas {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		display: block;
		touch-action: none;
	}
	.tag,
	.quiet,
	.pill {
		position: absolute;
		background: var(--tool-panel);
		text-shadow: var(--tool-halo);
		box-shadow: 0 0 0 var(--editor-edge) var(--tool-line);
	}
	.tag {
		top: calc(var(--shell-top) + 0.25rem);
		left: calc((var(--in-l) + 100% - var(--in-r)) / 2);
		transform: translateX(-50%);
		padding: 0.2rem 0.6rem;
		border-radius: 1rem;
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
		white-space: nowrap;
		max-width: calc(100% - var(--in-l) - var(--in-r));
		overflow: hidden;
		text-overflow: ellipsis;
		box-sizing: border-box;
	}
	.pill {
		display: flex;
		align-items: center;
		gap: 0.2rem;
		min-height: var(--control-height);
		padding: 0 0.25rem;
		border-radius: 1.2rem;
		white-space: nowrap;
		font-size: var(--editor-small);
	}
	.pill button {
		min-height: calc(var(--control-height) - 0.4rem);
		padding: 0 0.65rem;
		border: 0;
		border-radius: 1rem;
		background: transparent;
		color: inherit;
	}
	.pill button:hover {
		background: var(--tool-hover);
	}
	.pill .on {
		background: var(--tool-pressed);
		color: var(--tool-pressed-text);
		text-shadow: none;
	}
	.pill span.on {
		padding: 0.2rem 0.65rem;
		border-radius: 1rem;
	}
	.pill .round {
		width: calc(var(--control-height) - 0.4rem);
		padding: 0;
		display: inline-grid;
		place-items: center;
		border-radius: 50%;
	}
	.lab {
		padding: 0 0.4rem;
		color: var(--tool-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.sep {
		width: var(--editor-edge);
		height: 1.2rem;
		margin: 0 0.15rem;
		background: var(--tool-line);
	}
	.attach {
		top: calc(var(--shell-top) + 2.4rem);
		left: calc((var(--in-l) + 100% - var(--in-r)) / 2);
		transform: translateX(-50%);
		flex-wrap: wrap;
		justify-content: center;
		width: max-content;
		max-width: calc(100% - var(--in-l) - var(--in-r));
		box-sizing: border-box;
	}
	.quiet {
		left: calc(var(--in-l) + 0.25rem);
		bottom: calc(var(--status-room) + 2 * var(--control-height) + 2rem);
		max-width: 26rem;
		margin: 0;
		padding: 0.3rem 0.6rem;
		border-radius: var(--editor-radius);
		color: var(--tool-muted);
		font-size: var(--editor-small);
		line-height: 1.6;
	}
	.quiet.warn {
		font-weight: inherit;
		color: var(--tool-warn);
	}
	.clips {
		left: calc((var(--in-l) + 100% - var(--in-r)) / 2);
		bottom: calc(var(--status-room) + var(--control-height) + 1.25rem);
		transform: translateX(-50%);
		max-width: calc(100% - var(--in-l) - var(--in-r));
		overflow-x: auto;
	}
	.bar {
		position: relative;
		width: 10rem;
		height: 0.3rem;
		margin: 0 0.5rem;
		border-radius: 0.15rem;
		background: var(--tool-within);
		flex: none;
	}
	.bar i {
		position: absolute;
		left: 0;
		top: 0;
		bottom: 0;
		border-radius: 0.15rem;
		background: var(--tool-pressed);
	}
	.pill .quiet-button {
		color: var(--tool-muted);
	}
	.way-in {
		position: absolute;
		top: calc(var(--shell-top) + 3rem);
		left: calc(var(--in-l) + 2rem);
		right: calc(var(--in-r) + 2rem);
		max-width: 44rem;
		height: min(26rem, 60%);
		margin: 0 auto;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.8rem;
		border: var(--editor-edge) dashed var(--tool-muted);
		border-radius: 0.6rem;
		background: var(--tool-panel);
		text-shadow: var(--tool-halo);
		color: var(--tool-text);
		text-align: center;
	}
	.way-in .big {
		color: var(--tool-text);
		font-size: var(--editor-title, 1.15rem);
	}
	.way-in .row {
		display: flex;
		gap: 0.5rem;
	}
</style>
