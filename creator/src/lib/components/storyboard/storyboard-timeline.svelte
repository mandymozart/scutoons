<script lang="ts">
	// The storyboard page's timeline along the bottom, under the main view and the panels (#126; owner 2026-10-19: "the
	// timeline underneath the other windows was also good", "the other timeline underneath the clip only shows those shots
	// that are selected"; the studio's Timeline – research/TIMELINE.md): the shared @scutoons/ui Timeline with ONE lane –
	// the picked shots' clips back to back, each as long as its clip in the view mode (storyboard-timeline.ts); a clip
	// shows its frames one after another (clip-strip) and a tag with its source. The head: the view mode (like Blender's
	// viewport shading), the clips' count and length, "Back to the shot camera" while a take's moment holds the 3D view,
	// Play / Pause, the fold. A hover shows the clip's first frame next to the pointer. In Take mode the take's in / out
	// are the trim (way C). The AppShell's bottom region (storyboard-editor → usePageNav).
	import { FoldButton, Icon, IconButton, Segmented, Timeline } from '@scutoons/ui';
	import { formatTime } from '@scutoons/ui/timeline';
	import { shotSvg } from '@scutoons/content/storyboard';
	import { m } from '$lib/paraglide/messages.js';
	import { svgSource } from '$lib/editor/storyboard-shots.ts';
	import { MODES, shortGen, type ClipItem, type Mode, type Source } from '$lib/editor/storyboard-timeline.ts';
	import StatusBullet from '$lib/components/storyboard/status-bullet.svelte';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import ClipStrip from '$lib/components/storyboard/clip-strip.svelte';

	let { sb, onfold }: { sb: StoryboardPage; onfold: () => void } = $props();
	const tl = $derived(sb.tl);
	const v = $derived(tl.view);

	const SOURCE: Record<Source, { icon: string; word: () => string }> = {
		still: { icon: 'image', word: () => m['storyboard.tl_still']() },
		take: { icon: 'smartphone', word: () => m['storyboard.tl_take']() },
		rendered: { icon: 'film', word: () => m['storyboard.tl_rendered']() },
		generated: { icon: 'zap', word: () => m['storyboard.tl_generated']() }
	};
	const MODE_WORD: Record<Mode, [() => string, () => string]> = {
		shot: [() => m['storyboard.tl_mode_shot'](), () => m['storyboard.tl_mode_shot_hint']()],
		take: [() => m['storyboard.tl_mode_take'](), () => m['storyboard.tl_mode_take_hint']()],
		rendered: [() => m['storyboard.tl_mode_rendered'](), () => m['storyboard.tl_mode_rendered_hint']()],
		generated: [() => m['storyboard.tl_mode_generated'](), () => m['storyboard.tl_mode_generated_hint']()]
	};
	// the lane's words (the items' labels) in the creator's language
	$effect(() => {
		tl.words = { still: SOURCE.still.word(), take: SOURCE.take.word(), rendered: SOURCE.rendered.word(), generated: SOURCE.generated.word(), chosen: m['storyboard.tl_chosen'](), review: m['storyboard.tl_review'](), rejected: m['storyboard.tl_rejected'](), audition: m['storyboard.tl_audition']() };
	});
	const selected = $derived(v.sel ? [v.sel.id] : []);
	const reveal = $derived<[number, number] | undefined>(v.sel ? [v.sel.start, v.sel.start + v.sel.length] : undefined);
	const short = (id: string) => id.replace(/^sb-/, '');
	const asClip = (it: unknown) => it as ClipItem;
	/** a still / a take: the shot's schematic (a video: its own first frame) */
	function firstFrame(it: ClipItem): string | null {
		if (it.clip.gen !== undefined) return null;
		const e = sb.entries.find((x) => x.id === it.id);
		return e ? svgSource(shotSvg(e.shot, e.scene).svg) : null;
	}
</script>

{#snippet item(raw: import('@scutoons/ui').TimelineItem)}
	{@const it = asClip(raw)}
	{@const s = SOURCE[it.clip.source]}
	<ClipStrip {sb} {it} />
	<span class="tag" class:after-in={!!it.trim} style:--in={it.trim ? `${(it.trim.in / (it.length || 1)) * 100}%` : null} data-source={it.clip.source}>
		{#if it.clip.status}<StatusBullet status={it.clip.status} />{/if}<Icon name={s.icon} size="0.9em" /><span>{short(it.id)}</span><span class="word">{it.clip.gen !== undefined ? shortGen(it.clip) : s.word()}</span>
		{#if it.clip.audition}<span class="aud" title={m['storyboard.tl_audition_hint']()} data-testid="tl-audition">{m['storyboard.tl_audition']()}</span>{/if}
	</span>
{/snippet}

{#snippet preview(raw: import('@scutoons/ui').TimelineItem, _lane: unknown, t: number)}
	{@const it = asClip(raw)}
	{@const g = it.clip.gen}
	{@const still = firstFrame(it)}
	<span class="peek">
		{#if g !== undefined && tl.urls[g]}
			<video src="{tl.urls[g]}#t=0.001" preload="metadata" muted playsinline></video>
		{:else if still}
			<img src={still} alt="" />
		{/if}
		<span class="line">{it.label}</span>
		<span class="line">{m['storyboard.tl_at']({ at: formatTime(t - it.start, 's', 1), of: formatTime(it.length, 's', 1) })}</span>
	</span>
{/snippet}

<div class="sb-timeline" data-testid="storyboard-timeline">
	<header>
		<h2><Icon name="film" /> {m['storyboard.tl_title']()}</h2>
		<Segmented label={m['storyboard.tl_mode']()} radio options={MODES.map((x) => ({ value: x, label: MODE_WORD[x][0](), title: MODE_WORD[x][1]() }))} value={v.mode} onchange={(x) => tl.setMode(x)} testid="tl-mode" />
		<span class="cap">{m['storyboard.tl_count']({ n: v.items.length, length: formatTime(v.span) })}</span>
		<span class="grow"></span>
		{#if sb.hh.held && !sb.hh.live}
			<IconButton label={m['storyboard.tl_back']()} class="sc-round" onclick={() => sb.hh.endPreview()} data-testid="tl-back"><Icon name="video" /></IconButton>
		{/if}
		<IconButton label={tl.playing ? m['storyboard.tl_pause']() : m['storyboard.tl_play']()} class="sc-round" disabled={!v.items.length} onclick={() => tl.toggle()} data-testid="tl-play"><Icon name={tl.playing ? 'pause' : 'play'} /></IconButton>
		<FoldButton side="bottom" label={m['storyboard.tl_fold']()} controls="sc-shell-bottom" onclick={onfold} testid="tl-fold" />
	</header>
	{#if v.items.length}
		<Timeline
			label={m['storyboard.tl_title']()}
			lanes={[v.lane]}
			span={v.span}
			playhead={tl.playhead}
			{selected}
			bind:zoom={tl.zoom}
			{reveal}
			follow={tl.playing}
			{item}
			{preview}
			onseek={(t, final) => tl.seek(t, final)}
			onselect={(_lane, id) => tl.select(id)}
			ontrim={v.mode === 'take' ? (_lane, id, k, final, edge) => tl.trim(id, k, final, edge) : undefined}
		/>
	{:else}
		<p class="sc-empty">{m['storyboard.tl_empty']()}</p>
	{/if}
</div>

<style>
	.sb-timeline {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		background: var(--paper);
	}
	.sb-timeline > :global(:last-child) {
		flex: 1 1 auto;
		min-height: 0;
	}
	header {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		flex: none;
		padding: var(--space-05) var(--space-05) var(--space-025) var(--space-1);
		flex-wrap: wrap;
	}
	h2 {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		margin: 0;
		font-size: var(--type-label-size);
		line-height: var(--type-label-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		white-space: nowrap;
	}
	.cap {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--type-caption-size);
		font-variant-numeric: tabular-nums;
	}
	.grow {
		flex: 1;
	}
	.tag {
		position: absolute;
		left: 0;
		top: 0;
		display: inline-flex;
		align-items: center;
		gap: var(--space-025);
		max-width: 100%;
		padding: 0 var(--space-025);
		background: var(--paper);
		color: var(--ink);
		font-size: var(--type-caption-size);
		white-space: nowrap;
		overflow: hidden;
	}
	.tag.after-in {
		left: calc(var(--in) + 0.75rem);
	}
	.tag .word {
		opacity: 0.7;
	}
	/* the board's current card plays here: a small ink tag (a tag, not a status) */
	.aud {
		padding: 0 var(--space-025);
		background: var(--ink);
		color: var(--paper);
	}
	.peek {
		display: grid;
		gap: var(--space-025);
		padding: var(--space-05);
		font-size: var(--type-caption-size);
	}
	.peek video,
	.peek img {
		display: block;
		width: 10rem;
		max-height: 12rem;
		object-fit: contain;
		background: var(--paper);
	}
</style>
