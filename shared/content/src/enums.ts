export enum Renderer {
  VERTICAL = "vertical",
  HORIZONTAL = "horizontal",
}

export enum EventCategory {
	IRL = "irl",
	ONLINE = "virtual",
	HYBRID = "hybrid",
}

/** Content languages (a toon's `languages`); ISO 639-1. Korean is `ko` (2026-10-02, was the unused country code `kr`). */
export enum Language {
  DE = "de",
  EN = "en",
  JP = "jp",
  KO = "ko",
  ES = "es",
}

export enum Status {
  PUBLISHED = "published",
  DRAFT = "draft",
  COMING_SOON = "coming-soon",
}

export enum Trigger {
  INTERSECT = "intersect",
  HOVER = "hover",
  CLICK = "click",
}

export enum TriggerMode {
	MONO = "mono",
	POLY = "poly",
}

export enum TextBoxEffect {
	NONE = "none",
	SHIVER = "shiver",
	PULSE = "pulse",
	WOBBLE = "wobble",
	/** entrances (agents/FX.md): grows from nothing with an overshoot */
	POP = "pop",
	/** falls in from 3× size and lands with a thud, the slice shakes */
	SLAM = "slam",
	/** a soft blur-in */
	FADE = "fade",
	/** wiped in from the left with small key clicks */
	TYPE_ON = "typeOn",
}

export enum JumpToTargetType {
	CHAPTER = "chapter",
	SEASON = "season",
	TOON = "toon",
	PRODUCT = "product",
	XR = "xr",
}