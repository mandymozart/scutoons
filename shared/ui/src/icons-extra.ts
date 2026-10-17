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
	// text box sizing (Figma's three modes, drawn our way): auto width = a horizontal double arrow between two bars, auto
	// height = a vertical one, fixed size = a box with two text lines
	'fit-width': { contents: '<line x1="2" y1="6" x2="2" y2="18"/><line x1="22" y1="6" x2="22" y2="18"/><line x1="6" y1="12" x2="18" y2="12"/><polyline points="9 9 6 12 9 15"/><polyline points="15 9 18 12 15 15"/>' },
	'fit-height': { contents: '<line x1="6" y1="2" x2="18" y2="2"/><line x1="6" y1="22" x2="18" y2="22"/><line x1="12" y1="6" x2="12" y2="18"/><polyline points="9 9 12 6 15 9"/><polyline points="9 15 12 18 15 15"/>' },
	'fit-fixed': { contents: '<rect x="3" y="3" width="18" height="18"/><line x1="7" y1="10" x2="17" y2="10"/><line x1="7" y1="14" x2="14" y2="14"/>' },
	// outline width: a ring around a box
	'outline-width': { contents: '<rect x="3" y="3" width="18" height="18"/><rect x="8" y="8" width="8" height="8"/>' }
};
