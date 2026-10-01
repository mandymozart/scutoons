<script lang="ts" module>
	export interface ChartSeries {
		label: string;
		values: number[];
	}
</script>

<script lang="ts">
	/**
	 * Plain SVG line / bar chart in the SCUTOONS look (DESIGN.md): ink marks on paper, one y-axis from 0, recessive
	 * hairline grid, cyan only as a fill (bars of the first series, the area under the first line). Series identity
	 * never by color alone: bars = cyan fill / ink fill / ink outline / hatched; lines = solid / dashed / dotted /
	 * dash-dot, plus a legend for ≥ 2 series. Hover (or focus + arrow keys) shows a crosshair and the values of a day.
	 */
	let {
		kind = 'bar',
		labels,
		series,
		height = 180,
		ariaLabel,
		formatLabel = (label: string) => label,
		formatValue = (value: number) => String(value)
	}: {
		kind?: 'bar' | 'line';
		/** x labels (e.g. days), one per value */
		labels: string[];
		/** up to 4 series; more are not drawn (fold them into fewer) */
		series: ChartSeries[];
		height?: number;
		ariaLabel: string;
		formatLabel?: (label: string) => string;
		formatValue?: (value: number) => string;
	} = $props();

	const uid = $props.id();
	const W = 720;
	const pad = { top: 12, right: 12, bottom: 22, left: 36 };

	const shown = $derived(series.slice(0, 4));
	const n = $derived(labels.length);
	const rawMax = $derived(Math.max(0, ...shown.flatMap((s) => s.values)));
	/** a "nice" top: 1, 2, 5 × 10^k */
	const max = $derived.by(() => {
		if (rawMax <= 0) return 1;
		const pow = 10 ** Math.floor(Math.log10(rawMax));
		for (const step of [1, 2, 5, 10]) if (step * pow >= rawMax) return step * pow;
		return 10 * pow;
	});
	const plotW = W - pad.left - pad.right;
	const plotH = $derived(height - pad.top - pad.bottom);
	const band = $derived(plotW / Math.max(1, n));
	const y = (v: number) => pad.top + plotH - (v / max) * plotH;
	const xCenter = (i: number) => pad.left + band * i + band / 2;

	const barGap = $derived(band > 8 ? 2 : band > 4 ? 1 : 0);
	const barW = $derived(Math.max(1, (band - barGap * 2) / Math.max(1, shown.length) - (shown.length > 1 ? 1 : 0)));

	const linePath = (values: number[]) =>
		values.map((v, i) => `${i === 0 ? 'M' : 'L'}${xCenter(i).toFixed(1)},${y(v).toFixed(1)}`).join('');
	const areaPath = (values: number[]) =>
		values.length
			? `${linePath(values)}L${xCenter(values.length - 1).toFixed(1)},${y(0)}L${xCenter(0).toFixed(1)},${y(0)}Z`
			: '';

	const dash = ['', '6 4', '2 3', '8 3 2 3'];

	let hover = $state<number | null>(null);

	function move(event: PointerEvent) {
		const svg = event.currentTarget as SVGSVGElement;
		const box = svg.getBoundingClientRect();
		const x = ((event.clientX - box.left) / box.width) * W;
		const i = Math.floor((x - pad.left) / band);
		hover = i >= 0 && i < n ? i : null;
	}

	function keydown(event: KeyboardEvent) {
		if (!n) return;
		if (event.key === 'ArrowRight') hover = Math.min(n - 1, (hover ?? -1) + 1);
		else if (event.key === 'ArrowLeft') hover = Math.max(0, (hover ?? n) - 1);
		else if (event.key === 'Home') hover = 0;
		else if (event.key === 'End') hover = n - 1;
		else return;
		event.preventDefault();
	}

	const tipLeft = $derived(hover === null ? 0 : (xCenter(hover) / W) * 100);
	const ticks = $derived([0, max / 2, max]);
	const xTicks = $derived(n <= 1 ? [0] : n <= 3 ? [...Array(n).keys()] : [0, Math.floor((n - 1) / 2), n - 1]);
</script>

<figure class="sc-chart">
	{#if shown.length > 1}
		<ul class="legend">
			{#each shown as s, si (si)}
				<li>
					<svg width="22" height="12" aria-hidden="true">
						{#if kind === 'bar'}
							<rect x="1" y="1" width="20" height="10" class="bar s{si}" fill={si === 3 ? `url(#${uid}-hatch)` : undefined} />
						{:else}
							<line x1="1" x2="21" y1="6" y2="6" class="line" stroke-dasharray={dash[si]} />
						{/if}
					</svg>
					{s.label}
				</li>
			{/each}
		</ul>
	{/if}
	<div class="plot">
		<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
		<svg
			viewBox="0 0 {W} {height}"
			role="img"
			aria-label={ariaLabel}
			tabindex="0"
			onpointermove={move}
			onpointerleave={() => (hover = null)}
			onkeydown={keydown}
			onblur={() => (hover = null)}
		>
			<defs>
				<pattern id="{uid}-hatch" width="4" height="4" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
					<rect width="4" height="4" fill="var(--paper)" />
					<line x1="0" y1="0" x2="0" y2="4" stroke="var(--ink)" stroke-width="1.5" />
				</pattern>
			</defs>
			{#each ticks as t (t)}
				<line class="grid" x1={pad.left} x2={W - pad.right} y1={y(t)} y2={y(t)} />
				<text class="tick" x={pad.left - 6} y={y(t) + 4} text-anchor="end">{formatValue(t)}</text>
			{/each}
			{#if kind === 'bar'}
				{#each shown as s, si (si)}
					{#each s.values as v, i (i)}
						{#if v > 0}
							<rect
								class="bar s{si}"
								x={pad.left + band * i + barGap + si * (barW + (shown.length > 1 ? 1 : 0))}
								y={y(v)}
								width={barW}
								height={Math.max(0, y(0) - y(v))}
								fill={si === 3 ? `url(#${uid}-hatch)` : undefined}
							/>
						{/if}
					{/each}
				{/each}
			{:else}
				{#if shown[0]}
					<path class="area" d={areaPath(shown[0].values)} />
				{/if}
				{#each shown as s, si (si)}
					<path class="line" d={linePath(s.values)} stroke-dasharray={dash[si]} />
				{/each}
			{/if}
			<line class="axis" x1={pad.left} x2={W - pad.right} y1={y(0)} y2={y(0)} />
			{#each xTicks as i (i)}
				<text
					class="tick"
					x={xCenter(i)}
					y={height - 6}
					text-anchor={i === 0 && n > 1 ? 'start' : i === n - 1 && n > 1 ? 'end' : 'middle'}
				>
					{formatLabel(labels[i] ?? '')}
				</text>
			{/each}
			{#if hover !== null}
				<line class="crosshair" x1={xCenter(hover)} x2={xCenter(hover)} y1={pad.top} y2={y(0)} />
				{#if kind === 'line'}
					{#each shown as s, si (si)}
						<circle class="dot" cx={xCenter(hover)} cy={y(s.values[hover] ?? 0)} r="4" />
					{/each}
				{/if}
			{/if}
		</svg>
		{#if hover !== null}
			<div
				class="tip"
				style:left="{tipLeft}%"
				style:transform="translateX({tipLeft > 70 ? '-100%' : tipLeft < 30 ? '0' : '-50%'})"
				role="status"
			>
				<strong>{formatLabel(labels[hover] ?? '')}</strong>
				{#each shown as s (s.label)}
					<span>{s.label}: {formatValue(s.values[hover] ?? 0)}</span>
				{/each}
			</div>
		{/if}
	</div>
</figure>

<style>
	.sc-chart {
		margin: 0;
		font-variant-numeric: tabular-nums;
	}
	.legend {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-025) var(--space-1);
		margin: 0 0 var(--space-05);
		padding: 0;
		list-style: none;
		font-size: var(--type-caption-size);
	}
	.legend li {
		display: inline-flex;
		align-items: center;
		gap: var(--space-025);
	}
	.plot {
		position: relative;
	}
	svg[role='img'] {
		display: block;
		width: 100%;
		height: auto;
		overflow: visible;
	}
	svg[role='img']:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: 2px;
	}
	.grid {
		stroke: var(--border-subtle);
		stroke-width: 1;
	}
	.axis {
		stroke: var(--ink);
		stroke-width: 2;
	}
	.tick {
		fill: var(--text-muted);
		font-family: var(--font-mono);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.bar {
		stroke: var(--ink);
		stroke-width: 1;
	}
	.bar.s0 {
		fill: var(--primary-cyan);
	}
	.bar.s1 {
		fill: var(--ink);
	}
	.bar.s2 {
		fill: var(--paper);
	}
	.line {
		fill: none;
		stroke: var(--ink);
		stroke-width: 2;
		stroke-linejoin: round;
	}
	.area {
		fill: var(--primary-cyan);
		opacity: 0.35;
	}
	.crosshair {
		stroke: var(--ink);
		stroke-width: 1;
		stroke-dasharray: 2 2;
	}
	.dot {
		fill: var(--primary-cyan);
		stroke: var(--ink);
		stroke-width: 2;
	}
	.tip {
		position: absolute;
		top: 0;
		display: flex;
		flex-direction: column;
		gap: 0.1em;
		padding: var(--space-025) var(--space-05);
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		font-size: var(--type-caption-size);
		white-space: nowrap;
		pointer-events: none;
	}
</style>
