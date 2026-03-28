/**
 * @typedef {import('./models/enums').Renderer} RendererType
 * @typedef {import('./models/enums').Language} LanguageType
 * @typedef {import('./models/enums').Status} StatusType
 * @typedef {import('./models/enums').Trigger} TriggerType
 * @typedef {import('./models/enums').Interaction} InteractionType
 * @typedef {import('./models/enums').View} ViewType
 */

/**
 * @type {{VERTICAL: RendererType, HORIZONTAL: RendererType}}
 */
export const Renderer = {
  VERTICAL: /** @type {RendererType} */ ("vertical"),
  HORIZONTAL: /** @type {RendererType} */ ("horizontal"),
};

/**
 * @type {{DE: LanguageType, EN: LanguageType, JP: LanguageType, KR: LanguageType, ES: LanguageType}}
 */
export const Language = {
  DE: /** @type {LanguageType} */ ("de"),
  EN: /** @type {LanguageType} */ ("en"),
  JP: /** @type {LanguageType} */ ("jp"),
  KR: /** @type {LanguageType} */ ("kr"),
  ES: /** @type {LanguageType} */ ("es"),
};

/**
 * @type {{PUBLISHED: StatusType, DRAFT: StatusType, COMING_SOON: StatusType}}
 */
export const Status = {
  PUBLISHED: /** @type {StatusType} */ ("published"),
  DRAFT: /** @type {StatusType} */ ("draft"),
  COMING_SOON: /** @type {StatusType} */ ("coming-soon"),
};

/**
 * @type {{SCROLL_IN_VIEW: TriggerType, HOVER: TriggerType, CLICK: TriggerType}}
 */
export const Trigger = {
  SCROLL_IN_VIEW: /** @type {TriggerType} */ ("scrollInView"),
  HOVER: /** @type {TriggerType} */ ("hover"),
  CLICK: /** @type {TriggerType} */ ("click"),
};

/**
 * @type {{WEBTOON: InteractionType, VISUAL_NOVEL: InteractionType}}
 */
export const Interaction = {
  WEBTOON: /** @type {InteractionType} */ ("webtoon"),
  VISUAL_NOVEL: /** @type {InteractionType} */ ("visual-novel"),
};

/**
 * @type {{TOON: ViewType, CHAPTER: ViewType, SONG: ViewType, ACCOUNT: ViewType, CREDITS: ViewType}}
 */
export const View = {
  TOON: /** @type {ViewType} */ ("toon"),
  CHAPTER: /** @type {ViewType} */ ("chapter"),
  SONG: /** @type {ViewType} */ ("song"),
  ACCOUNT: /** @type {ViewType} */ ("account"),
  CREDITS: /** @type {ViewType} */ ("credits"),
};
