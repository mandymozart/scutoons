<!--
	An entrance of a showTextBox event (agents/FX.md): the text box it reveals comes in once – pop, slam, fade or typeOn –
	with its sound, then stays like the other effects; `onComplete` after delay + duration.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import type { Snippet } from 'svelte';
	import type { FxSoundChoice } from '@scutoons/content';
	import { buzz, playFxSound, playRevealSound } from '../fx/sounds.ts';
	import { impactCue } from '../fx/cues.ts';

	let {
		kind,
		duration = 1000,
		delay = 0,
		onComplete,
		sound,
		children
	}: {
		kind: 'pop' | 'slam' | 'fade' | 'typeOn';
		duration?: number;
		delay?: number;
		onComplete: () => void;
		/** the event's sound (none set = the entrance's own, `none` = silent) */
		sound?: FxSoundChoice;
		children: Snippet;
	} = $props();

	let el: HTMLDivElement;
	let visible = $state(false);

	function enter() {
		visible = true;
		if (kind === 'pop') {
			playRevealSound(sound, 'pop');
			buzz(15);
			el.animate(
				[
					{ transform: 'scale(0)', opacity: 0 },
					{ transform: 'scale(1.25)', opacity: 1, offset: 0.6 },
					{ transform: 'scale(0.95)', offset: 0.8 },
					{ transform: 'scale(1)' }
				],
				{ duration: 380, easing: 'ease-out' }
			);
		} else if (kind === 'slam') {
			el.animate(
				[
					{ transform: 'scale(3) rotate(-8deg)', opacity: 0 },
					{ transform: 'scale(0.9) rotate(0deg)', opacity: 1, offset: 0.7 },
					{ transform: 'scale(1)' }
				],
				{ duration: 260, easing: 'cubic-bezier(.5,0,.9,.4)' }
			);
			setTimeout(() => {
				playRevealSound(sound, 'thud');
				buzz(40);
				impactCue('shake', { slice: el.closest<HTMLElement>('.slice'), sound: null, duration: 320 });
			}, 180);
		} else if (kind === 'fade') {
			playRevealSound(sound, 'soft');
			el.animate([{ opacity: 0, filter: 'blur(6px)' }, { opacity: 1, filter: 'blur(0)' }], { duration: 700, easing: 'ease-out' });
		} else typedMs = typeOn();
	}
	let typedMs = 0;

	/**
	 * typeOn (owner 2026-10-07): the box opens like an old screen – a tiny square, a thin line across, then the full
	 * height – and only then its text is typed letter by letter with key clicks (or the event's one sound). Returns
	 * when it has finished (ms from now).
	 */
	let typing: ReturnType<typeof setInterval> | undefined;
	function typeOn(): number {
		// the text nodes as drawn; emptied now, typed back below (the same strings – nothing else changes)
		const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
		const nodes: { node: Text; text: string }[] = [];
		for (let n = walker.nextNode(); n; n = walker.nextNode()) if ((n as Text).data.trim()) nodes.push({ node: n as Text, text: (n as Text).data });
		for (const t of nodes) t.node.data = '';
		const OPEN = [60, 220, 220];
		const opened = OPEN.reduce((a, b) => a + b, 0);
		el.animate(
			[
				{ clipPath: 'inset(calc(50% - 1px) calc(50% - 1px))', offset: 0 },
				{ clipPath: 'inset(calc(50% - 1px) calc(50% - 1px))', offset: OPEN[0] / opened },
				{ clipPath: 'inset(calc(50% - 1px) 0)', offset: (OPEN[0] + OPEN[1]) / opened },
				{ clipPath: 'inset(0)', offset: 1 }
			],
			{ duration: opened, easing: 'ease-out' }
		);
		const total = nodes.reduce((a, t) => a + [...t.text].length, 0);
		const step = Math.max(18, Math.min(55, 2500 / Math.max(1, total)));
		setTimeout(() => {
			if (sound) playRevealSound(sound);
			let node = 0;
			let at = 0;
			typing = setInterval(() => {
				const t = nodes[node];
				if (!t) return clearInterval(typing);
				const chars = [...t.text];
				at++;
				t.node.data = chars.slice(0, at).join('');
				if (!sound && chars[at - 1]?.trim()) playFxSound('key');
				if (at >= chars.length) {
					node++;
					at = 0;
				}
			}, step);
		}, opened);
		return opened + total * step;
	}

	onMount(() => {
		let durationTimer: ReturnType<typeof setTimeout> | undefined;
		const delayTimer = setTimeout(() => {
			enter();
			// typeOn takes as long as its text needs (at least the event's duration)
			durationTimer = setTimeout(onComplete, kind === 'typeOn' ? Math.max(duration, typedMs) : duration);
		}, delay);
		return () => {
			clearTimeout(delayTimer);
			clearTimeout(durationTimer);
			clearInterval(typing);
		};
	});
</script>

<div class="effect-enter" class:visible bind:this={el}>
	{@render children()}
</div>

<style>
	/* fills the text box it reveals */
	.effect-enter {
		display: block;
		width: 100%;
		height: 100%;
		visibility: hidden;
	}

	.effect-enter.visible {
		visibility: visible;
	}
</style>
