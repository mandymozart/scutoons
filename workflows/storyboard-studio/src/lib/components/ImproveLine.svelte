<script>
	// The line the Generate panel starts with while a board card fills it (agents/research/VARIANTS-CANVAS.md §3.6, issue
	// #62 stage 3 – improve.svelte.js): the card's thumbnail (a video's first frame – posters.js), "Improve on g2" / "Vary
	// g2", how the card goes in for the model now (improve.js howText), a generator switched because its own takes no such
	// card (said here), a still set by hand / from More ▾ over the card's (named here), ✕ (round: back to the shot's own
	// inputs and the generator before – the focus then on the panel's first control); under it the SEED – New (the
	// server picks one) or the card's own – where the model takes a seed and the card has one.
	import { tick } from 'svelte';
	import { Icon, IconButton, PropRow, Segmented } from '@scutoons/ui';
	import { at, clearFrom, current, gen, model, setSeed, stills } from '../generate.svelte.js';
	import { chosenImage, howText, stillLabel } from '../improve.js';
	import { devicePx, posterOf, pxStep } from '../posters.js';

	const f = $derived(gen.from);
	const m = $derived(model());
	let poster = $state(null);
	$effect(() => {
		const [url, video] = [f?.url, f?.video];
		poster = null;
		if (!url || !video) return;
		let alive = true;
		posterOf(url, pxStep(devicePx(3))).then((p) => alive && (poster = p));
		return () => (alive = false);
	});
	/** a picture that didn't load (the render before its first save) – no broken image */
	let broken = $state('');
	const thumb = $derived(f?.video ? poster : f?.url ?? null);
	const label = (k) => stills().find((s) => s.key === k)?.label ?? stillLabel(k, gen.shot);
	const how = $derived(howText(f, m, { chosen: chosenImage(stills()), at: at(), label }));
	/** ✕: the line goes – the focus to the panel's first control (it would fall out of the panel) */
	let el = $state();
	function back() {
		const panel = el?.parentElement;
		clearFrom();
		tick().then(() => panel?.querySelector('select:not(:disabled), button:not(:disabled), textarea, [tabindex="0"]')?.focus());
	}
	const switched = $derived(f?.switched && gen.form.generator === f.switched.to ? `${current()?.label} – ${f.switched.why}` : '');
	const title = $derived(`${f?.mode === 'vary' ? 'Vary' : 'Improve on'} ${f?.name ?? ''}`);
</script>

{#if f}
	<div class="line" bind:this={el} data-testid="improve-line">
		<span class="pic">{#if thumb && thumb !== broken}<img src={thumb} alt="" draggable="false" onerror={() => (broken = thumb)} />{/if}</span>
		<span class="words" title="{title} – the board's card fills the form; Generate asks with the cost before anything is sent">
			<span class="name">{title}</span>
			{#if how}<span class="muted">{how}</span>{/if}
			{#if switched}<span class="muted" data-testid="improve-switched">{switched}</span>{/if}
		</span>
		<IconButton label="Back to {f.shot}'s own inputs{f.prev && f.prev.generator !== gen.form.generator ? ' and the generator before' : ''}" class="sc-round" onclick={back} data-testid="improve-clear"><Icon name="x" /></IconButton>
	</div>
	{#if f.seed != null && m?.seed}
		<PropRow label="Seed" hint="New: the server picks one (the record keeps it) · {f.name}'s: the same seed again – with another prompt, a controlled change">
			<Segmented label="Seed" options={[{ value: 'new', label: 'New', title: 'A new seed – the server picks one' }, { value: 'own', label: String(f.seed), title: `${f.name}'s seed` }]} value={f.useSeed ? 'own' : 'new'} onchange={(v) => setSeed(v === 'own')} testid="improve-seed" />
		</PropRow>
	{/if}
{/if}

<style>
	.line {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		min-width: 0;
	}
	/* the card's picture, a control and a half high, in its aspect (letterboxed in ink) */
	.pic {
		flex: none;
		display: block;
		width: 3rem;
		height: 3rem;
		background: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		box-sizing: border-box;
	}
	img {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
	.words {
		flex: 1;
		min-width: 0;
		display: grid;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		overflow-wrap: anywhere;
	}
	.name {
		font-size: var(--type-button-size);
		line-height: var(--type-button-line);
	}
	.muted {
		color: var(--text-muted);
	}
</style>
