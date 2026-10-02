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

export enum OnomatopoeiaVariant {
	DEFAULT = "default",
	PULSE = "outline",
	WOBBLE = "bubble",
	THORNS = "thorns"
}

export enum OnomatopoeiaEffect {
	NONE = "none",
	SHIVER = "shiver",
	PULSE = "pulse",
	WOBBLE = "wobble",
}

export enum JumpToTargetType {
	CHAPTER = "chapter",
	SEASON = "season",
	TOON = "toon",
	PRODUCT = "product",
	XR = "xr",
}