<script lang="ts">
	import { tick, type Snippet } from 'svelte';
	import Icon from '../icon.svelte';
	import { closesOnResize, menuPlace, menuStep, openFocus } from './tool-groups.ts';

	/**
	 * A stage toolbar group that is one button opening a small panel of settings attached to it (grid & snap, view): the
	 * button shows the group's icon (a corner mark says it opens more; `marked` adds a dot – snapping on), the panel holds
	 * the group's rows. A tap opens it with the panel focused, ↓ / ↑ on the button with its first control focused; ↑ ↓
	 * (Home / End) move between its buttons, a field keeps its own keys, Esc closes and returns the focus, Tab out of it or
	 * a tap outside closes it. Fixed to the screen, kept on it.
	 */
	let {
		label,
		title,
		icon,
		marked = false,
		children
	}: {
		label: string;
		/** What the group holds now (the button's tooltip and name) */
		title: string;
		icon: string;
		marked?: boolean;
		children: Snippet;
	} = $props();

	const menuId = $props.id();
	let shown = $state(false);
	let trigger = $state<HTMLButtonElement>();
	let menu = $state<HTMLDivElement>();
	let at = $state({ left: 0, top: 0 });
	let widthAtOpen = 0;

	const focusables = (): HTMLElement[] =>
		menu ? [...menu.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled)')] : [];

	export const open = async (byKeyboard = false): Promise<void> => {
		if (!trigger) return;
		widthAtOpen = window.innerWidth;
		const r = trigger.getBoundingClientRect();
		at = { left: r.left, top: r.bottom + 2 };
		shown = true;
		await tick();
		if (menu) {
			const m = menu.getBoundingClientRect();
			at = menuPlace(r, m, { width: window.innerWidth, height: window.innerHeight }, !!trigger.closest('[aria-orientation="vertical"], .vertical'));
		}
		if (openFocus(byKeyboard) === 'first') focusables()[0]?.focus();
		else menu?.focus({ preventScroll: true });
	};
	const close = (refocus: boolean) => {
		shown = false;
		if (refocus) trigger?.focus();
	};
	$effect(() => {
		if (!shown) return;
		const down = (event: PointerEvent) => {
			const target = event.target as Element | null;
			if (target && (menu?.contains(target) || trigger?.contains(target))) return;
			close(false);
		};
		window.addEventListener('pointerdown', down, true);
		return () => window.removeEventListener('pointerdown', down, true);
	});
	const triggerKey = (event: KeyboardEvent) => {
		if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
		event.preventDefault();
		event.stopPropagation();
		void open(true);
	};
	const menuKey = (event: KeyboardEvent) => {
		if (event.key === 'Escape') {
			event.preventDefault();
			event.stopPropagation();
			close(true);
			return;
		}
		const tag = (event.target as HTMLElement).tagName;
		if (tag === 'INPUT' || tag === 'SELECT') {
			event.stopPropagation();
			return;
		}
		const all = focusables().filter((el) => el.tagName === 'BUTTON');
		const index = all.indexOf(document.activeElement as HTMLElement);
		const key = event.key === 'ArrowLeft' || event.key === 'ArrowRight' ? null : event.key;
		const next = key ? (index < 0 && all.length ? 0 : menuStep(key, index, all.length)) : null;
		event.stopPropagation();
		if (next === null) return;
		event.preventDefault();
		all[next]?.focus();
	};
	const menuFocusOut = (event: FocusEvent) => {
		const next = event.relatedTarget as Node | null;
		if (next && !menu?.contains(next) && next !== trigger) close(false);
	};
</script>

<svelte:window
	onresize={() => {
		if (shown && closesOnResize(widthAtOpen, window.innerWidth)) close(false);
	}}
/>

<span class="tool-group" role="group" aria-label={label}>
	<button
		bind:this={trigger}
		type="button"
		class="tool"
		class:open={shown}
		class:marked
		aria-haspopup="dialog"
		aria-expanded={shown}
		aria-controls={shown ? menuId : undefined}
		{title}
		aria-label="{label}: {title}"
		onclick={() => (shown ? close(false) : void open())}
		onkeydown={triggerKey}
	>
		<Icon name={icon} size="1rem" strokeWidth={2} />
	</button>
	{#if shown}
		<div
			bind:this={menu}
			id={menuId}
			class="flyout"
			role="dialog"
			aria-label={label}
			tabindex="-1"
			style:left="{at.left}px"
			style:top="{at.top}px"
			onkeydown={menuKey}
			onfocusout={menuFocusOut}
		>
			{@render children()}
		</div>
	{/if}
</span>

<style>
	.tool-group {
		flex: none;
		display: inline-flex;
		align-items: center;
	}
	.tool {
		position: relative;
		display: inline-grid;
		place-items: center;
		width: var(--kit-h, var(--control-height));
		height: var(--kit-h, var(--control-height));
		padding: 0;
		border: 0;
		/* in a Pill: round and concentric with it (--kit-h / --kit-r), as the kit's IconButton */
		border-radius: var(--kit-r, var(--editor-radius));
		background: var(--kit-icon-button-rest-ground);
		color: var(--kit-icon-button-rest-text);
		cursor: pointer;
	}
	.tool::after {
		content: '';
		position: absolute;
		right: 0.125rem;
		bottom: 0.125rem;
		border: 0.15rem solid transparent;
		border-right-color: var(--tool-muted);
		border-bottom-color: var(--tool-muted);
	}
	.tool.marked::before {
		content: '';
		position: absolute;
		right: 0.2rem;
		top: 0.2rem;
		width: 0.3rem;
		height: 0.3rem;
		border-radius: 50%;
		background: currentColor;
	}
	.tool.open {
		background: var(--tool-pressed);
		color: var(--tool-pressed-text);
	}
	.tool:focus-visible {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: -2px;
	}
	@media (hover: hover) {
		.tool:not(.open):hover {
			background: var(--tool-hover);
		}
	}
	.flyout {
		position: fixed;
		z-index: 2000;
		display: grid;
		gap: 0.375rem;
		width: max-content;
		max-width: calc(100vw - 0.5rem);
		padding: 0.5rem var(--editor-pad);
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: var(--editor-radius);
		background: var(--tool-popover);
		color: var(--tool-text);
		outline: none;
		text-shadow: none;
	}
</style>
