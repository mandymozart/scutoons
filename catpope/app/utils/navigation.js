import { sanitizeNumber } from './template.js';

const MAX_CHAPTER = 999;
const MAX_SONG = 999;

export const VALID_VIEWS = ['toon', 'chapter', 'song', 'account', 'credits'];

export class NavigationParams {
	constructor(urlParams = new URLSearchParams(window.location.search)) {
		this.view = this.parseView(urlParams.get('view'));
		this.chapter = sanitizeNumber(urlParams.get('chapter'), 1, 1, MAX_CHAPTER);
		this.song = sanitizeNumber(urlParams.get('song'), 1, 1, MAX_SONG);
	}

	parseView(view) {
		return VALID_VIEWS.includes(view) ? view : 'toon';
	}

	toURLSearchParams() {
		const params = new URLSearchParams();
		if (this.view !== 'toon') params.set('view', this.view);
		if (this.chapter > 1) params.set('chapter', this.chapter);
		if (this.song > 1) params.set('song', this.song);
		return params;
	}

	toString() {
		const params = this.toURLSearchParams();
		return params.toString() ? `?${params.toString()}` : '';
	}
}

export function parseURL() {
	return new NavigationParams();
}

export function updateURL(params) {
	const current = parseURL();
	const urlParams = new URLSearchParams();
	
	// Use provided params, fall back to current URL values
	const view = params.view || current.view;
	const chapter = params.chapter !== undefined 
		? sanitizeNumber(params.chapter, 1, 1, MAX_CHAPTER) 
		: current.chapter;
	const song = params.song !== undefined 
		? sanitizeNumber(params.song, 1, 1, MAX_SONG) 
		: current.song;
	
	// Only add non-default values to URL
	if (view !== 'toon') urlParams.set('view', view);
	if (view === 'chapter' && chapter > 0) urlParams.set('chapter', chapter);
	if (view === 'song' && song > 0) urlParams.set('song', song);
	
	const newURL = window.location.pathname + (urlParams.toString() ? `?${urlParams.toString()}` : '');
	window.history.pushState(params, '', newURL);
	
	return { view, chapter, song };
}

export function navigateToView(scutoon, view, params = {}) {
	const navParams = updateURL({ view, ...params });
	return renderView(scutoon, navParams);
}

export async function renderView(scutoon, navParams) {
	scutoon.navigationStore.setView(navParams.view, {
		chapter: navParams.chapter,
		song: navParams.song
	});

	switch (navParams.view) {
		case 'chapter':
			return await scutoon.renderChapterView(navParams.chapter);
		case 'song':
			return await scutoon.renderSongView(navParams.song);
		case 'account':
			return scutoon.renderAccountView();
		case 'credits':
			return await scutoon.renderCreditsView();
		default:
			return await scutoon.renderToonView();
	}
}
