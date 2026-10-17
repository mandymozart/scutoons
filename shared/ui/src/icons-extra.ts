// Icons Feather doesn't have, drawn on its 24 grid in its manner (strokes only, `currentColor`; DESIGN.md "Icons": a
// missing icon goes into shared/ui, never into an app). `Icon` looks here after Feather. Trusted markup, not user input.

export const EXTRA_ICONS: Record<string, { contents: string }> = {
	// vertical alignment: the box's top and bottom edge, two text lines at the top / in the middle / at the bottom
	'valign-top': { contents: '<line x1="3" y1="3" x2="21" y2="3"/><line x1="3" y1="21" x2="21" y2="21"/><line x1="7" y1="8" x2="17" y2="8"/><line x1="7" y1="12" x2="17" y2="12"/>' },
	'valign-middle': { contents: '<line x1="3" y1="3" x2="21" y2="3"/><line x1="3" y1="21" x2="21" y2="21"/><line x1="7" y1="10" x2="17" y2="10"/><line x1="7" y1="14" x2="17" y2="14"/>' },
	'valign-bottom': { contents: '<line x1="3" y1="3" x2="21" y2="3"/><line x1="3" y1="21" x2="21" y2="21"/><line x1="7" y1="12" x2="17" y2="12"/><line x1="7" y1="16" x2="17" y2="16"/>' },
	// letter spacing: two letters' stems with a double arrow under them
	'letter-spacing': { contents: '<line x1="8" y1="3" x2="8" y2="13"/><line x1="16" y1="3" x2="16" y2="13"/><line x1="3" y1="19" x2="21" y2="19"/><polyline points="6 16 3 19 6 22"/><polyline points="18 16 21 19 18 22"/>' },
	// line height: a double arrow up / down beside two text lines
	'line-height': { contents: '<line x1="5" y1="3" x2="5" y2="21"/><polyline points="2 6 5 3 8 6"/><polyline points="2 18 5 21 8 18"/><line x1="11" y1="8" x2="21" y2="8"/><line x1="11" y1="16" x2="21" y2="16"/>' },
	// outline width: a ring around a box
	'outline-width': { contents: '<rect x="3" y="3" width="18" height="18"/><rect x="8" y="8" width="8" height="8"/>' }
};
