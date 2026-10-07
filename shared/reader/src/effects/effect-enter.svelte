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
		} else {
			// typed: wiped in from the left in one step per letter, a small key click per step (or the event's one sound)
			const letters = Math.max(1, Math.min(60, (el.textContent ?? '').replace(/\s+/g, '').length));
			const ms = Math.min(Math.max(letters * 55, 300), Math.max(300, duration));
			el.animate([{ clipPath: 'inset(-50% 100% -50% -50%)' }, { clipPath: 'inset(-50% -50% -50% -50%)' }], { duration: ms, easing: `steps(${letters}, end)` });
			if (sound) playRevealSound(sound);
			else {
				let i = 0;
				const step = ms / letters;
				const timer = setInterval(() => {
					if (++i > letters) return clearInterval(timer);
					playFxSound('key');
				}, step);
			}
		}
	}

	onMount(() => {
		let durationTimer: ReturnType<typeof setTimeout> | undefined;
		const delayTimer = setTimeout(() => {
			enter();
			durationTimer = setTimeout(onComplete, duration);
		}, delay);
		return () => {
			clearTimeout(delayTimer);
			clearTimeout(durationTimer);
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
