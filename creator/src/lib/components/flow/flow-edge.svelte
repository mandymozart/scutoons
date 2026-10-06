<script lang="ts" module>
	/** where dagre put the edge: its bends between the boxes and its label's centre (flow coordinates) */
	export interface FlowEdgeData extends Record<string, unknown> {
		bends: { x: number; y: number }[];
		labelAt?: { x: number; y: number };
	}
</script>

<script lang="ts">
	import { BaseEdge, type Edge, type EdgeProps } from '@xyflow/svelte';

	/**
	 * An edge of the Flow graph along dagre's route: from the source box's bottom through the bends dagre laid out to the
	 * target's top, its label where dagre made room for it (a label is a node of its own in dagre's layout, so the labels
	 * of edges leaving the same box sit side by side instead of on top of each other at the boxes' midpoint).
	 */
	let { id, sourceX, sourceY, targetX, targetY, data, label, markerEnd, style, interactionWidth }: EdgeProps<Edge<FlowEdgeData>> = $props();

	/** a smooth path through the points (Catmull-Rom as cubic Béziers) */
	function through(points: { x: number; y: number }[]): string {
		let d = `M ${points[0].x},${points[0].y}`;
		for (let i = 0; i < points.length - 1; i++) {
			const p0 = points[i - 1] ?? points[i];
			const p1 = points[i];
			const p2 = points[i + 1];
			const p3 = points[i + 2] ?? p2;
			const c1 = { x: p1.x + (p2.x - p0.x) / 6, y: p1.y + (p2.y - p0.y) / 6 };
			const c2 = { x: p2.x - (p3.x - p1.x) / 6, y: p2.y - (p3.y - p1.y) / 6 };
			d += ` C ${c1.x},${c1.y} ${c2.x},${c2.y} ${p2.x},${p2.y}`;
		}
		return d;
	}

	const points = $derived([{ x: sourceX, y: sourceY }, ...(data?.bends ?? []), { x: targetX, y: targetY }]);
	const path = $derived(through(points));
	const at = $derived(data?.labelAt ?? { x: (sourceX + targetX) / 2, y: (sourceY + targetY) / 2 });
</script>

<BaseEdge {id} {path} {label} labelX={at.x} labelY={at.y} {markerEnd} {style} {interactionWidth} />
