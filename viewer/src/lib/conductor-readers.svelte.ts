// #529: what the conductor page draws of the core's linked readers – the page's link sets it (live-vote-link.ts
// `onReaders`, relay-web `readers()`), the linked-reader panel reads it. A copy of the core's answer, nothing derived.
import type { LinkedReaders } from './conductor-reader.ts';

export class ReadersView {
	now = $state.raw<LinkedReaders | null>(null);
	readonly set = (r: LinkedReaders): void => void (this.now = r);
}
