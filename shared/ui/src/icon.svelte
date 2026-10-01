<script lang="ts">
	/**
	 * A UI icon from Feather (feather-icons, MIT), the house icon set for tools and controls (agents/DESIGN.md "Icons").
	 * Use icons where a word makes no sense (map tools, zoom, close, drag); keep text where a word is clearer.
	 * The brand's own hand-drawn icons (viewer: chapter, map, inventory …) stay as they are.
	 * Decorative by default (aria-hidden); give `label` when the icon is the only content of a control.
	 */
	import feather from 'feather-icons';

	type Props = {
		/** a Feather icon name, e.g. "map-pin", "zoom-in", "x" (https://feathericons.com) */
		name: string;
		/** CSS size; default 1.25em (scales with the fluid root type) */
		size?: string;
		/** stroke width; the brutalist default is 2.5 (Feather's own is 2) */
		strokeWidth?: number;
		/** accessible name; omit for decorative icons next to a text label */
		label?: string;
	};

	let { name, size = '1.25em', strokeWidth = 2.5, label }: Props = $props();

	// the icon's inner SVG markup comes from the bundled feather-icons data (trusted, not user input)
	const icon = $derived(feather.icons[name as keyof typeof feather.icons]);
</script>

{#if icon}
	<svg
		class="icon"
		xmlns="http://www.w3.org/2000/svg"
		viewBox="0 0 24 24"
		width={size}
		height={size}
		fill="none"
		stroke="currentColor"
		stroke-width={strokeWidth}
		stroke-linecap="square"
		stroke-linejoin="miter"
		role={label ? 'img' : undefined}
		aria-label={label}
		aria-hidden={label ? undefined : 'true'}
		data-icon={name}
	>
		{@html icon.contents}
	</svg>
{/if}

<style>
	.icon {
		display: inline-block;
		flex: none;
		vertical-align: middle;
	}
</style>
