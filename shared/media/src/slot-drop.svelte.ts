// A slot's side of a drag: the host says whether the dragged file fits, the slot lights or shows why not.
import { mediaHost } from './setup.ts';
import type { DropAnswer, SlotApply } from './slot-host.ts';
import type { SlotFit } from './library/types.ts';

export class SlotDrop {
	/** the host's answer while a file is over the slot */
	answer = $state<DropAnswer | null>(null);

	constructor(
		private fit: () => SlotFit,
		private apply: SlotApply
	) {}

	over = (event: DragEvent) => {
		const answer = mediaHost()?.over(event, this.fit()) ?? null;
		this.answer = answer;
		if (!answer?.ok) return;
		event.preventDefault();
		if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
	};

	leave = () => {
		this.answer = null;
	};

	drop = (event: DragEvent) => {
		this.answer = null;
		mediaHost()?.drop(event, this.fit(), this.apply);
	};
}
