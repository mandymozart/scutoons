<script lang="ts">
	// individual goes here and style is delegated to the components in the sub folder onomatopeia

	import type { VNActiveOnomatopoeia } from '$lib/contexts/visual-novel.svelte.ts';
	import { OnomatopoeiaVariant, OnomatopoeiaEffect } from '$lib/models/enums.ts';
	import { sanitizeNumber } from '$lib/utils/template.ts';

	import EffectNone from './onomatopeia/effects/effect-none.svelte';
	import EffectShiver from './onomatopeia/effects/effect-shiver.svelte';
	import EffectPulse from './onomatopeia/effects/effect-pulse.svelte';
	import EffectWobble from './onomatopeia/effects/effect-wobble.svelte';

	import OnomatopeiaDefault from './onomatopeia/variants/onomatopeia-default.svelte';
	import OnomatopeiaOutline from './onomatopeia/variants/onomatopeia-outline.svelte';
	import OnomatopeiaBubble from './onomatopeia/variants/onomatopeia-bubble.svelte';
	import OnomatopeiaThorns from './onomatopeia/variants/onomatopeia-thorns.svelte';

	let { onomatopeia }: { onomatopeia: VNActiveOnomatopoeia } = $props();

	const normalizedText = Array.isArray(onomatopeia.text) ? onomatopeia.text : [onomatopeia.text];

	const effectComponents = {
		[OnomatopoeiaEffect.NONE]: EffectNone,
		[OnomatopoeiaEffect.SHIVER]: EffectShiver,
		[OnomatopoeiaEffect.PULSE]: EffectPulse,
		[OnomatopoeiaEffect.WOBBLE]: EffectWobble
	};

	const variantComponents = {
		[OnomatopoeiaVariant.DEFAULT]: OnomatopeiaDefault,
		[OnomatopoeiaVariant.PULSE]: OnomatopeiaOutline,
		[OnomatopoeiaVariant.WOBBLE]: OnomatopeiaBubble,
		[OnomatopoeiaVariant.THORNS]: OnomatopeiaThorns
	};

	const EffectComponent = effectComponents[onomatopeia.effect ?? OnomatopoeiaEffect.NONE];
	const VariantComponent = variantComponents[onomatopeia.variant ?? OnomatopoeiaVariant.DEFAULT];
	
	const safeDuration = sanitizeNumber(onomatopeia.duration ?? 2000, 2000, 0, 5000);
	const safeDelay = sanitizeNumber(onomatopeia.delay ?? 0, 0, 0, 5000);
	
	$inspect(onomatopeia);
</script>

<div
	class="onomatopeia"
	style="left: {onomatopeia.position?.x ?? 50}%; top: {onomatopeia.position?.y ?? 50}%;"
>
	<EffectComponent
		duration={safeDuration}
		delay={safeDelay}
		onComplete={onomatopeia.onComplete}
	>
		<VariantComponent
			text={normalizedText}
			size={onomatopeia.size}
			align={onomatopeia.align}
			rotation={onomatopeia.rotation}
		/>
	</EffectComponent>
</div>

<style>
	.onomatopeia {
		position: absolute;
		transform: translate(-50%, -50%);
		pointer-events: none;
		z-index: 100;
		color: #000;
	}
</style>
