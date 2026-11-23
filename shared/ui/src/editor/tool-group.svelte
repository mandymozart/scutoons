<script lang="ts" module>
	import type { ToolOption } from './tool-groups.ts';

	export type ToolGroupOption = ToolOption & { icon: string };
</script>

<script lang="ts">
	import { tick } from 'svelte';
	import Icon from '../icon.svelte';
	import { activeIndex, activeOption, menuPlace, menuStep } from './tool-groups.ts';

	/**
	 * One group of a stage toolbar: collapsed, one button with the active option's icon (a corner mark says it opens more)
	 * whose menu of options attaches to it; expanded, the options side by side as pressed buttons. The menu is a WAI-ARIA
	 * menu (↑ ↓ ← →, Home / End, Enter / Space choose, Esc or Tab close), fixed to the screen and kept on it; a tap outside
	 * closes it. With `selected` the group is a set of toggles: each option on or off, the menu stays open after a choice.
	 */
	let {
		label,
		options,
		value = '',
		selected,
		icon,
		expanded = false,
		onchoose
	}: {
		label: string;
		options: readonly ToolGroupOption[];
		value?: string;
		selected?: readonly string[];
		/** Toggles, collapsed: the button's icon unless exactly one option is on */
		icon?: string;
		expanded?: boolean;
		onchoose: (value: string) => void;
	} = $props();

	const toggles = $derived(selected !== undefined);
	const active = $derived(activeOption(options, selected ? (selected[0] ?? '') : value));
	const isOn = (option: ToolGroupOption) => (selected ? selected.includes(option.value) : option.value === active.value);
	const onNames = $derived(selected ? options.filter((o) => selected.includes(o.value)).map((o) => o.name) : []);
	const buttonIcon = $derived(!selected || selected.length === 1 ? active.icon : (icon ?? active.icon));
	const buttonName = $derived(!selected ? active.name : onNames.length ? onNames.join(', ') : '–');
	const menuId = $props.id();
	let open = $state(false);
	let trigger = $state<HTMLButtonElement>();
	let menu = $state<HTMLDivElement>();
	let at = $state({ left: 0, top: 0 });
	const items: HTMLButtonElement[] = $state([]);

	const focusItem = (index: number) => items[index]?.focus();
	const show = async () => {
		if (!trigger) return;
		const r = trigger.getBoundingClientRect();
		at = { left: r.left, top: r.bottom + 2 };
		open = true;
		await tick();
		if (menu) {
			const m = menu.getBoundingClientRect();
			at = menuPlace(r, m, { width: window.innerWidth, height: window.innerHeight }, !!trigger.closest('[aria-orientation="vertical"], .vertical'));
		}
		focusItem(toggles ? 0 : activeIndex(options, value));
	};
	const close = (refocus: boolean) => {
		open = false;
		if (refocus) trigger?.focus();
	};
	const choose = (next: string) => {
		onchoose(next);
		if (!toggles) close(true);
	};
	$effect(() => {
		if (!open) return;
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
		void show();
	};
	const menuKey = (event: KeyboardEvent) => {
		if (event.key === 'Escape') {
			event.preventDefault();
			event.stopPropagation();
			close(true);
			return;
		}
		if (event.key === 'Tab') return close(false);
		const index = items.indexOf(document.activeElement as HTMLButtonElement);
		const next = menuStep(event.key, index < 0 ? activeIndex(options, value) : index, options.length);
		if (next === null) return;
		event.preventDefault();
		event.stopPropagation();
		focusItem(next);
	};
</script>

<svelte:window
	onresize={() => {
		if (open) close(false);
	}}
/>

<span class="tool-group" role="group" aria-label={label}>
	{#if expanded}
		{#each options as option (option.value)}
			<button
				type="button"
				class="tool"
				class:on={isOn(option)}
				aria-pressed={isOn(option)}
				title={option.label}
				aria-label={option.label}
				onclick={() => onchoose(option.value)}><Icon name={option.icon} size="1rem" strokeWidth={2} /></button
			>
		{/each}
	{:else}
		<button
			bind:this={trigger}
			type="button"
			class="tool more"
			class:open
			aria-haspopup="menu"
			aria-expanded={open}
			aria-controls={open ? menuId : undefined}
			title="{label}: {buttonName}"
			aria-label="{label}: {buttonName}"
			onclick={() => (open ? close(false) : void show())}
			onkeydown={triggerKey}
		>
			<Icon name={buttonIcon} size="1rem" strokeWidth={2} />
		</button>
		{#if open}
			<!-- svelte-ignore a11y_interactive_supports_focus -->
			<div bind:this={menu} id={menuId} class="flyout" role="menu" aria-label={label} tabindex="-1" style:left="{at.left}px" style:top="{at.top}px" onkeydown={menuKey}>
				{#each options as option, i (option.value)}
					<button
						bind:this={items[i]}
						type="button"
						class="item"
						class:on={isOn(option)}
						role={toggles ? 'menuitemcheckbox' : 'menuitemradio'}
						aria-checked={isOn(option)}
						tabindex={(toggles ? i === 0 : option.value === active.value) ? 0 : -1}
						title={option.label}
						onclick={() => choose(option.value)}
					>
						<Icon name={option.icon} size="1rem" strokeWidth={2} /><span class="name">{option.name}</span>{#if option.key}<kbd>{option.key}</kbd>{/if}
					</button>
				{/each}
			</div>
		{/if}
	{/if}
</span>

<style>
	.tool-group {
		flex: none;
		display: inline-flex;
		align-items: center;
		gap: 0.125rem;
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
	.tool.on,
	.tool.open {
		background: var(--kit-icon-button-on-ground);
		color: var(--kit-icon-button-on-text);
		text-shadow: none;
	}
	.tool.on :global(svg),
	.tool.open :global(svg) {
		filter: none;
	}
	.more::after {
		content: '';
		position: absolute;
		right: 0.125rem;
		bottom: 0.125rem;
		border: 0.15rem solid transparent;
		border-right-color: var(--tool-muted);
		border-bottom-color: var(--tool-muted);
	}
	.flyout {
		position: fixed;
		z-index: 2000;
		display: flex;
		flex-direction: column;
		min-width: 9rem;
		padding: 0.25rem;
		border: var(--editor-edge) solid var(--kit-menu-rest-border);
		border-radius: var(--kit-radius-box);
		background: var(--kit-menu-rest-ground);
		color: var(--tool-text);
		outline: none;
		text-shadow: var(--tool-halo);
	}
	.item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		height: var(--control-height);
		padding: 0 0.625rem;
		border: none;
		border-radius: var(--editor-radius);
		background: none;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
		white-space: nowrap;
	}
	.item.on {
		background: var(--kit-list-row-selected-ground);
		color: var(--kit-list-row-selected-text);
	}
	.name {
		flex: 1;
	}
	kbd {
		font: inherit;
		font-size: var(--editor-small);
		color: var(--tool-muted);
	}
	.tool:focus-visible,
	.item:focus-visible {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: -2px;
	}
	@media (hover: hover) {
		.tool:not(.on, .open):hover {
			background: var(--kit-icon-button-hover-ground);
		}
		.item:not(.on):hover {
			background: var(--kit-menu-item-hover-ground);
		}
	}
</style>
