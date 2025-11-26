import type { VNEvent } from '$lib/models/visual-novel.ts';
import type { GameContext } from '$lib/contexts/game.svelte';
import type { VisualNovelContext } from '$lib/contexts/visual-novel.svelte';

export class EventProcessor {
	private events: VNEvent[];
	private game: GameContext;
	private vn: VisualNovelContext;
	private currentIndex: number = 0;

	constructor(events: VNEvent[] | undefined, game: GameContext, vn: VisualNovelContext) {
		this.events = events || [];
		this.game = game;
		this.vn = vn;
	}

	async run(): Promise<string | null> {
		console.log(`🎬 Processing ${this.events.length} VN events`);

		while (this.currentIndex < this.events.length) {
			const event = this.events[this.currentIndex];
			console.log(
				`Event ${this.currentIndex + 1}/${this.events.length}: ${event.type}`,
				event.params
			);

			const result = await this.processEvent(event);

			if (result?.jumpTo) {
				console.log(`⏭️ Jumping to: ${result.jumpTo}`);
				return result.jumpTo;
			}

			this.currentIndex++;
		}

		console.log('✅ All VN events processed');
		return null;
	}

	private async processEvent(event: VNEvent): Promise<{ jumpTo?: string } | void> {
		switch (event.type) {
			case 'showMessage':
				return await this.handleShowMessage(event);

			case 'hideMessage':
				return this.handleHideMessage(event);

			case 'showChoices':
				return await this.handleShowChoices(event);

			case 'addChoice':
				return await this.handleAddChoice(event);

			case 'joinScene':
				return this.handleJoinScene(event);

			case 'leaveScene':
				return this.handleLeaveScene(event);
			
			case 'showOnomatopeia':
				return this.handleShowOnomatopoeia(event);

			case 'end':
				return { jumpTo: event.params.jumpTo };

			default:
				console.warn(`Unknown event type: ${(event as any).type}`);
		}
	}

	private interpolateVariables(text: string): string {
		return text.replace(/{(\w+)}/g, (match, varName) => {
			const value = this.game.getVariable(varName);
			if (value === undefined) {
				console.warn(`Variable "${varName}" not found in game state`);
				return match;
			}
			return String(value);
		});
	}

	private async handleShowMessage(
		event: VNEvent & { type: 'showMessage' }
	): Promise<void> {
		const { text, character: characterName, wait = false } = event.params;

		let charRef = null;
		if (characterName) {
			const inScene = this.vn.state.sceneCharacters.find((c) => c.name === characterName);
			if (!inScene) {
				console.error(
					`Character "${characterName}" is not in the scene. Add them with joinScene first.`
				);
				return;
			}
			charRef = inScene;
		}

		const interpolatedText = this.interpolateVariables(text);
		this.vn.showMessage(charRef, interpolatedText, wait);

		if (wait) {
			return new Promise<void>((resolve) => {
				this.vn.setWaitResolver(resolve);
			});
		}
	}

	private handleHideMessage(event: VNEvent & { type: 'hideMessage' }): void {
		this.vn.hideMessage();
	}

	private async handleShowChoices(
		event: VNEvent & { type: 'showChoices' }
	): Promise<void> {
		this.vn.showChoices();

		return new Promise((resolve) => {
			const checkChoices = () => {
				if (this.vn.state.pendingChoices.length === 0) {
					resolve();
				} else {
					setTimeout(checkChoices, 50);
				}
			};
			checkChoices();
		});
	}

	private handleAddChoice(event: VNEvent & { type: 'addChoice' }): void {
		const { label, onSelect } = event.params;
		const interpolatedLabel = this.interpolateVariables(label);
		this.vn.addChoice({ label: interpolatedLabel, onSelect });
	}

	private handleJoinScene(event: VNEvent & { type: 'joinScene' }): void {
		const { characters } = event.params;
		const charsWithDefaults = characters.map((char) => ({
			...char,
			position: char.position || { variant: 'avatar' as const }
		}));
		this.vn.joinScene(charsWithDefaults);
	}

	private handleLeaveScene(event: VNEvent & { type: 'leaveScene' }): void {
		const { characters } = event.params;
		this.vn.leaveScene(characters);
	}

	private async handleShowOnomatopoeia(event: VNEvent & { type: 'showOnomatopeia' }): Promise<void> {
		const { wait = false } = event.params;
		
		this.vn.showOnomatopeia(event.params);

		if (wait) {
			return new Promise<void>((resolve) => {
				this.vn.setOnomatopoeiaResolver(resolve);
			});
		}
	}
}
