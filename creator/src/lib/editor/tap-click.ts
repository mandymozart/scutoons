/**
 * A finger's (or pen's) tap is followed by a click where it lifted. When the tap itself opened something there – the
 * slice canvas's item / piece tool opens its picker on pointerup – that click lands on whatever the picker shows under
 * the finger and picks it (owner 2026-10-18, the phone: "It should work exactly like the items" – the item tool placed
 * the item card that lay under the finger). This eats that one click; a mouse's click goes to the element it was
 * pressed on, so only `pointerType` touch / pen need it. `ms`: how long to wait for it (a page without
 * width=device-width delays it ~300 ms).
 */
export function swallowTapClick(event: PointerEvent, ms = 600) {
	if (event.pointerType === 'mouse') return;
	const eat = (e: MouseEvent) => {
		e.preventDefault();
		e.stopImmediatePropagation();
		stop();
	};
	const stop = () => {
		window.removeEventListener('click', eat, true);
		clearTimeout(timer);
	};
	const timer = setTimeout(stop, ms);
	window.addEventListener('click', eat, true);
}
