<script lang="ts">
	import { Icon, Segmented } from '@scutoons/ui';
	import { COVER_ALIGNS, COVER_COLORS, COVER_COLOR_VALUES, COVER_LIMITS, COVER_TEXT_SOURCES, type CoverLine } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { alignLabel, colorLabel, sourceLabel } from '$lib/editor/cover-words.ts';
	import MediaSlot from '$lib/components/media/media-slot.svelte';
	import ReorderButtons from '$lib/components/controls/reorder-buttons.svelte';

	/**
	 * The cover editor's side: the art (a slot → the asset picker), the lettering lines (select, add) and the selected
	 * line's properties – its text (toon / season title or an own text per language), quotes, position / size / width /
	 * angle / skew, alignment, colours, order, remove.
	 */
	let {
		toon,
		place,
		art,
		mode,
		lines,
		selected = $bindable(null),
		customLanguages,
		lineLabel,
		onart,
		onadd,
		onedit,
		onnumber,
		ontext,
		oncustom,
		onmove,
		onremove
	}: {
		toon: string;
		place: string;
		art: string | undefined;
		mode: 'off' | 'default' | 'custom';
		lines: CoverLine[];
		selected?: number | null;
		/** the languages an own text is written in (the header's text language) */
		customLanguages: string[];
		lineLabel: (l: CoverLine) => string;
		onart: (path: string | undefined) => void;
		onadd: () => void;
		onedit: (index: number, change: Partial<CoverLine>, key: string) => void;
		onnumber: (index: number, key: keyof typeof COVER_LIMITS, raw: string) => void;
		ontext: (index: number, text: CoverLine['text']) => void;
		oncustom: (index: number, locale: string, value: string) => void;
		onmove: (from: number, to: number) => void;
		onremove: (index: number) => void;
	} = $props();

	const line = $derived(selected !== null ? lines[selected] : undefined);
</script>

<aside class="inspector" data-testid="cover-inspector">
	<!-- the art as a slot (the preview is the button → the asset picker; the path only as the tooltip) -->
	<div class="art" data-testid="cover-art-field" title={m['creator.cover_art_hint']()}>
		<MediaSlot {toon} {place} label={m['creator.cover_art']()} value={art} onchange={onart} testid="cover-art-slot" />
	</div>
	{#if mode !== 'off'}
		<p class="sc-label">{m['creator.cover_lines']()}</p>
		<ul class="lines">
			{#each lines as l, i (i)}
				<li>
					<button type="button" class="ghost row" class:active={selected === i} onclick={() => (selected = i)} data-testid="cover-line-{i}">
						<span class="src">{sourceLabel(l.text)}</span>
						<span class="txt">{lineLabel(l)}</span>
					</button>
				</li>
			{/each}
		</ul>
		<button type="button" class="secondary" onclick={onadd} data-testid="cover-add-line"><Icon name="plus" /> {m['creator.cover_add_line']()}</button>
	{/if}

	{#if line && selected !== null}
		{@const i = selected}
		<div class="props" data-testid="cover-props">
			<label class="field">
				<span class="sc-label">{m['creator.cover_text']()}</span>
				<select value={line.text} onchange={(e) => ontext(i, e.currentTarget.value as CoverLine['text'])} data-testid="cover-text-source">
					{#each COVER_TEXT_SOURCES as s (s)}<option value={s} selected={line.text === s}>{sourceLabel(s)}</option>{/each}
				</select>
			</label>
			{#if line.text === 'custom'}
				<div class="custom" data-testid="cover-custom-text">
					{#each customLanguages as l (l)}
						<label class="field">
							<span class="sc-label">{l}</span>
							<input type="text" value={line.custom?.[l] ?? ''} lang={l} oninput={(e) => oncustom(i, l, e.currentTarget.value)} data-testid="cover-custom-{l}" />
						</label>
					{/each}
				</div>
			{:else}
				<p class="hint">{m['creator.cover_text_hint']()}</p>
			{/if}
			<label class="check">
				<input type="checkbox" checked={line.quotes === true} onchange={(e) => onedit(i, { quotes: e.currentTarget.checked || undefined }, 'cover-quotes')} />
				<span>{m['creator.cover_quotes']()}</span>
			</label>
			<div class="sc-fields" data-cols="2" data-density="panel">
				{#each [['x', m['creator.cover_x']()], ['y', m['creator.cover_y']()], ['size', m['creator.cover_size']()], ['maxWidth', m['creator.cover_width']()], ['rotate', m['creator.cover_rotate']()], ['skew', m['creator.cover_skew']()]] as [key, label] (key)}
					{@const k = key as keyof typeof COVER_LIMITS}
					<label class="field">
						<span class="sc-label">{label}</span>
						<input type="number" step="0.5" min={COVER_LIMITS[k].min} max={COVER_LIMITS[k].max} value={line[k] ?? 0} onchange={(e) => onnumber(i, k, e.currentTarget.value)} data-testid="cover-{key}" />
					</label>
				{/each}
			</div>
			<div class="field">
				<span class="sc-label">{m['creator.cover_align']()}</span>
				<Segmented
					label={m['creator.cover_align']()}
					value={line.align ?? 'center'}
					onchange={(a) => onedit(i, { align: a === 'center' ? undefined : a }, 'cover-align')}
					options={COVER_ALIGNS.map((a) => ({ value: a, icon: `align-${a}`, title: alignLabel(a) }))}
				/>
			</div>
			<div class="sc-fields" data-cols="2" data-density="panel">
				{#each [['color', m['creator.cover_color'](), 'paper'], ['outline', m['creator.cover_outline'](), 'ink']] as [key, label, fallback] (key)}
					{@const k = key as 'color' | 'outline'}
					<label class="field">
						<span class="sc-label">{label}</span>
						<span class="color">
							<span class="swatch" style="background: {COVER_COLOR_VALUES[line[k] ?? (fallback as 'paper')]}"></span>
							<select onchange={(e) => onedit(i, { [k]: e.currentTarget.value === fallback ? undefined : e.currentTarget.value }, `cover-${k}`)} data-testid="cover-{key}">
								{#each COVER_COLORS as c (c)}<option value={c} selected={(line[k] ?? fallback) === c}>{colorLabel(c)}</option>{/each}
							</select>
						</span>
					</label>
				{/each}
			</div>
			<div class="line-actions">
				<ReorderButtons index={i} count={lines.length} {onmove} variant="secondary" labels={{ up: m['creator.cover_line_up'](), down: m['creator.cover_line_down']() }} />
				<button type="button" class="secondary" title={m['creator.cover_remove_line']()} onclick={() => onremove(i)} data-testid="cover-remove-line"><Icon name="trash-2" /> {m['creator.cover_remove_line']()}</button>
			</div>
		</div>
	{/if}
</aside>

<style>
	.inspector {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		min-width: 0;
	}
	.inspector > .sc-label {
		margin: 0;
	}
	.hint {
		margin: 0;
	}
	.lines {
		list-style: none;
		margin: 0;
		padding: 0;
		border: var(--border-width-regular) solid var(--ink);
	}
	.lines li + li {
		border-top: var(--border-width-hairline) solid var(--ink);
	}
	.row {
		width: 100%;
		justify-content: flex-start;
		gap: var(--space-05);
		text-align: left;
	}
	.row .src {
		flex: none;
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.row .txt {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.props {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		border-top: var(--border-width-regular) solid var(--ink);
		padding-top: var(--space-1);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
	}
	/* the alignment control as wide as its three buttons */
	.field > :global(.sc-seg) {
		align-self: flex-start;
	}
	.custom {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	.check {
		display: flex !important;
		align-items: center;
		gap: var(--space-05);
	}
	.color {
		display: flex;
		align-items: stretch;
		gap: var(--space-05);
	}
	.swatch {
		flex: none;
		width: var(--control-height);
		border: var(--border-width-regular) solid var(--ink);
		box-sizing: border-box;
	}
	.line-actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
	.art {
		padding-bottom: var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
</style>
