<script lang="ts" module>
	export type DesignerTabId = 'parameters' | 'nodes' | 'rust' | 'assignment';
</script>

<script lang="ts">
	// The designer's tabs in the creator's Sound design workspace: Parameters · Nodes · Rust · Assignment (RIG-EDITING.md
	// §2), the kit's pill tabs – no border at rest, the open one in the text colour (RULES H4).
	import { PillTabs, type PillTabItem } from '@scutoons/ui/editor';
	import { m } from '../messages.ts';

	let { value = 'rust', onchange }: { value?: DesignerTabId; onchange: (id: DesignerTabId) => void } = $props();

	const ids: DesignerTabId[] = ['parameters', 'nodes', 'rust', 'assignment'];
	const label = { parameters: m['designer.tab_parameters'], nodes: m['designer.tab_nodes'], rust: m['designer.tab_rust'], assignment: m['designer.tab_assignment'] };
	const items = $derived<PillTabItem[]>(ids.map((id) => ({ id, label: label[id](), testid: `designer-tab-${id}` })));
</script>

<div class="tabs" data-testid="designer-tabs">
	<PillTabs {items} on={value} label={m['designer.tabs']()} onselect={(id) => onchange(id as DesignerTabId)} />
</div>

<style>
	.tabs {
		flex: none;
		display: flex;
		min-width: 0;
		overflow-x: auto;
		scrollbar-width: none;
		padding: 0 var(--space-05);
	}
</style>
