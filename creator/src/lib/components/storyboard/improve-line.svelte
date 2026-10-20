<script lang="ts">
	// The line the Generate form starts with while a board card fills it (#125; VARIANTS-CANVAS §3.6 – the port of #62's
	// ImproveLine): "Improve on g2" / "Vary g2", how the card goes in for the model now (storyboard-improve.ts howOf), a
	// generator switched because its own takes no such card (said here), ✕ (round: back to the shot's own inputs and the
	// generator before); under it the SEED – New (the provider picks one) or the card's – where the model takes a seed and
	// the card has one. Another model picked: the card's fields for it (the prompt stays as edited); another shot: the
	// line goes (the form follows the shot).
	import { untrack } from 'svelte';
	import { CloseIcon, IconButton, PropRow, Segmented } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { howOf, takesSeed, type HowCode } from '$lib/editor/storyboard-improve.ts';
	import { shortName } from '$lib/editor/storyboard-board-layout.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb }: { sb: StoryboardPage } = $props();
	const imp = $derived(sb.board.improve);
	const f = $derived(imp.from);
	const gen = $derived(sb.gen);
	// the form follows the model: the card's fields again
	$effect(() => {
		void gen.form.model;
		untrack(() => imp.refit());
	});
	// another shot: the line goes
	$effect(() => {
		const shot = sb.current;
		untrack(() => {
			if (imp.from && imp.from.shot !== shot) imp.from = null;
		});
	});
	const chosen = $derived(f ? (sb.generations.find((g) => g.shot === f.shot && g.kind === 'image' && g.approved)?.id ?? null) : null);
	const how = $derived(howOf(f, gen.model, chosen));
	const name = $derived(f ? f.name || (f.as === 'take' ? m['storyboard.board_take']({ take: f.take ?? '' }) : m['storyboard.board_render']()) : '');
	const stillName = $derived.by(() => {
		const id = f?.still ?? null;
		const g = id === null ? null : sb.generations.find((x) => x.id === id);
		return g ? shortName(g) : '';
	});
	const HOW: Record<HowCode, () => string> = {
		start: () => m['storyboard.improve_how_start']({ name }),
		edit: () => m['storyboard.improve_how_edit']({ name }),
		reference: () => m['storyboard.improve_how_reference']({ name }),
		video_key: () => m['storyboard.improve_how_video_key']({ name }),
		video_nokey: () => m['storyboard.improve_how_video_nokey']({ name }),
		own: () => m['storyboard.improve_how_own'](),
		vary: () => m['storyboard.improve_how_vary'](),
		set: () => m['storyboard.improve_how_set']({ still: stillName }),
		none: () => m['storyboard.improve_how_none']({ name })
	};
	const switched = $derived(f?.switched && gen.form.generator === f.switched.to ? m['storyboard.improve_switched']({ generator: gen.current?.label ?? f.switched.to }) : '');
	const title = $derived(f ? (f.mode === 'vary' ? m['storyboard.vary_title']({ name }) : m['storyboard.improve_title']({ name })) : '');
</script>

{#if f}
	<div class="line" data-testid="improve-line">
		<span class="words" title={m['storyboard.improve_line_hint']()}>
			<span class="name">{title}</span>
			{#if how}<span class="muted" data-testid="improve-how">{HOW[how]()}</span>{/if}
			{#if switched}<span class="muted" data-testid="improve-switched">{switched}</span>{/if}
		</span>
		<IconButton label={m['storyboard.improve_clear']()} class="sc-round" onclick={() => imp.clear()} data-testid="improve-clear"><CloseIcon /></IconButton>
	</div>
	{#if f.seed != null && takesSeed(gen.model)}
		<PropRow label={m['storyboard.improve_seed']()} hint={m['storyboard.improve_seed_hint']()}>
			<Segmented label={m['storyboard.improve_seed']()} options={[{ value: 'new', label: m['storyboard.improve_seed_new']() }, { value: 'own', label: String(f.seed) }]} value={f.useSeed ? 'own' : 'new'} onchange={(v) => imp.setSeed(v === 'own')} testid="improve-seed" />
		</PropRow>
	{/if}
{/if}

<style>
	.line {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		min-width: 0;
		padding-bottom: var(--space-05);
		border-bottom: var(--border-width-hairline) solid var(--ink);
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
