/**
 * Taps and pointer events on these (and inside them) are app UI, not input to the AR scene or the stage.
 * `data-app-ui` marks a whole region as UI (the editor's panels, bars and menus, #131).
 */
export const UI_SELECTOR = 'button, a, input, select, textarea, label, [role="button"], [role="listbox"], [role="option"], [role="dialog"], [data-app-ui]';

export const onAppUi = (event: Event): boolean =>
  event.composedPath().some(el => el instanceof Element && el.matches(UI_SELECTOR));
