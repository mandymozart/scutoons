<script lang="ts">
	// one onomatopoeia; style is delegated to the variant and effect components next to this file
	import { OnomatopoeiaVariant, OnomatopoeiaEffect, sanitizeNumber } from '@scutoons/content';
	import type { ActiveOnomatopoeia } from '../types.ts';

	import EffectNone from './effects/effect-none.svelte';
	import EffectShiver from './effects/effect-shiver.svelte';
	import EffectPulse from './effects/effect-pulse.svelte';
	import EffectWobble from './effects/effect-wobble.svelte';

	import OnomatopoeiaDefault from './variants/onomatopoeia-default.svelte';
	import OnomatopoeiaOutline from './variants/onomatopoeia-outline.svelte';
	import OnomatopoeiaBubble from './variants/onomatopoeia-bubble.svelte';
	import OnomatopoeiaThorns from './variants/onomatopoeia-thorns.svelte';

	let { onomatopoeia }: { onomatopoeia: ActiveOnomatopoeia } = $props();

	const normalizedText = $derived(Array.isArray(onomatopoeia.text) ? onomatopoeia.text : [onomatopoeia.text]);

	const effectComponents = {
		[OnomatopoeiaEffect.NONE]: EffectNone,
		[OnomatopoeiaEffect.SHIVER]: EffectShiver,
		[OnomatopoeiaEffect.PULSE]: EffectPulse,
		[OnomatopoeiaEffect.WOBBLE]: EffectWobble
	};

	const variantComponents = {
		[OnomatopoeiaVariant.DEFAULT]: OnomatopoeiaDefault,
		[OnomatopoeiaVariant.PULSE]: OnomatopoeiaOutline,
		[OnomatopoeiaVariant.WOBBLE]: OnomatopoeiaBubble,
		[OnomatopoeiaVariant.THORNS]: OnomatopoeiaThorns
	};

	const EffectComponent = $derived(effectComponents[onomatopoeia.effect ?? OnomatopoeiaEffect.NONE]);
	const VariantComponent = $derived(variantComponents[onomatopoeia.variant ?? OnomatopoeiaVariant.DEFAULT]);

	const safeDuration = $derived(sanitizeNumber(onomatopoeia.duration ?? 2000, 2000, 0, 5000));
	const safeDelay = $derived(sanitizeNumber(onomatopoeia.delay ?? 0, 0, 0, 5000));
	/** uniform scale around the anchor, on top of `size` (rotation is drawn by the variant) */
	const scale = $derived(
		typeof onomatopoeia.scale === 'number' && onomatopoeia.scale >= 0.1 && onomatopoeia.scale <= 10 ? onomatopoeia.scale : 1
	);
</script>

<div
	class="onomatopeia"
	style="left: {onomatopoeia.position?.x ?? 50}%; top: {onomatopoeia.position?.y ?? 50}%;"
	style:transform={scale !== 1 ? `translate(-50%, -50%) scale(${scale})` : null}
>
	<EffectComponent
		duration={safeDuration}
		delay={safeDelay}
		onComplete={onomatopoeia.onComplete}
	>
		<VariantComponent
			text={normalizedText}
			size={onomatopoeia.size}
			align={onomatopoeia.align}
			rotation={onomatopoeia.rotation}
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
