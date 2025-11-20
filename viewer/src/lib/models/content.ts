import { Renderer, Language, Status, Interaction, type EventCategory } from './enums.ts';
import type { AudioEvents, AudioConfiguration } from './audio.ts';
import type { VNEvent } from './visual-novel.ts';
import type { SwiperOptions } from 'swiper/types';

export interface KeyVisual {
	image: string;
	video?: string;
}

export interface Dialog {
	rect: {
		x: number;
		y: number;
		width: number;
	};
	text: string[];
}

export interface Onomatopoeia {
	position: {
		x: number;
		y: number;
		size: number;
		align: 'left' | 'right' | 'center';
	};
	rotation: number;
	text: string[];
}

export interface Slice {
	image: string;
	audio?: AudioEvents;
	dialog?: Dialog[];
	onomatopoeia?: Onomatopoeia[];
	_type?: string;
	_label?: string;
}

export interface RendererParams {
	autoPlay?: {
		enabled?: boolean;
		delay?: number;
	};
	loop?: boolean;
	flip?: boolean;
	keyboard?: boolean;
	cube?: boolean;
	freeMode?: boolean;
}

export interface Group {
	label?: string;
	renderer?: {
		type: Renderer, 
		params?: RendererParams
	};
	interaction?: Interaction;
	slices?: Slice[];
	events?: VNEvent[];
	ref?: RefLink;
}

export interface RefLink {
	id: string;
	src: string;
	type: 'chapter' | 'group' | 'song'; // db type
}

export interface Chapter {
	title: string;
	author: string;
	number: number;
	language: Language;
	published: boolean;
	release: string;
	status: Status;
	description: string;
	thumbnail: string;
	keyvisual?: KeyVisual;
	audio?: AudioConfiguration;
	groups: Group[];
	isPreview?: boolean;
}

export interface ChapterWithId extends Chapter {
	id: number;
}

export interface Song {
	title: string;
	author?: string;
	number: number;
	language: Language;
	published: boolean;
	release: string;
	status: Status;
	description: string;
	thumbnail: string;
	src: string;
	isPreview?: boolean;
}

export interface SongWithId extends Song {
	id: number;
}

export interface Cover {
	square?: string;
	portrait?: string;
}

export interface Credits {
	artist?: string[];
	writer?: string;
	music?: string[];
	sound?: string[];
}

export interface Person {
	name: string;
	roles: string[];
}

export interface PerformanceCast {
	name: string;
	venue: string;
	date: string;
	conductor?: string;
	members: Person[];
}

export interface SongCredits {
	title: string;
	credits: Person[];
}

export interface FullCredits {
	general: Person[];
	songs: SongCredits[];
	performances: PerformanceCast[];
	specialThanks: string[];
}

export interface SequenceItem {
	type: 'song' | 'chapter';
	number: number;
}

export interface Season {
	title: string;
	authors?: string[];
	language?: string;
	number?: number;
	release?: string;
	description?: string;
	cover?: Cover;
	credits?: Credits;
	sequence: SequenceItem[];
}

export interface Event {
	title: string;
	date: string;
	location: string;
	city: string;
	category: EventCategory;
	link?: string;
}

export interface Download {
	complete: string | null;
	music: string;
}

export interface Toon {
	title: string;
	authors: string[];
	chapters: number;
	language: string;
	release: string;
	status: Status;
	description: string;
	thumbnail: string;
	cover: Cover;
	download: Download;
}

export interface ToonIndex {
	id: string;
	uri: string;
	status: Status;
	avatar: string;
	i18n: Record<string, Record<string, string>>;
}

export type LoadedSequenceItem = (Chapter | Song) & {
	type: 'chapter' | 'song';
	number: number;
};
