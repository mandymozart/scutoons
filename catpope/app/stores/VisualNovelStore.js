/**
 * @typedef {import('../types').Character} Character
 * @typedef {import('../types').Language} LanguageType
 */

import { loadCharacters } from "../utils/api.js";

export class VisualNovelStore {
  /** @type {Map<string, Character>} */
  characters = new Map();
  /** @type {string} */
  toonId;
  /** @type {LanguageType} */
  currentLang;
  /** @type {Array<{el: HTMLButtonElement, handler: (this: HTMLButtonElement, ev: PointerEvent) => any}>} */
  pendingChoices = [];

  /**
   * @param {string} toonId
   * @param {LanguageType} lang
   */
  constructor(toonId, lang) {
    this.toonId = toonId;
    this.currentLang = lang;
    // Load characters immediately
    this.loadCharacters(lang);
  }

  /**
   * @param {LanguageType} lang
   * @returns {Promise<void>}
   */
  async loadCharacters(lang) {
    // Reload if language changed
    if (this.characters.size > 0 && lang === this.currentLang) {
      return; // Already loaded for this language
    }

    this.currentLang = lang;
    this.characters.clear();

    try {
      const characters = await loadCharacters(lang);
      characters.forEach((char) => {
        this.characters.set(char.name, char);
      });
      console.log(`Loaded ${this.characters.size} characters for ${lang}`);
    } catch (error) {
      console.error("Failed to load characters:", error);
    }
  }

  /**
   * @param {string} name
   * @returns {Character | undefined}
   */
  getCharacter(name) {
    return this.characters.get(name);
  }

  /** @param {HTMLButtonElement} el
   * @param {(this: HTMLButtonElement, ev: PointerEvent) => any} handler */
  addChoice(el, handler) {
    this.pendingChoices.push({ el, handler });
  }

  /**
   * @param {HTMLButtonElement} el
   * @param {(this: HTMLButtonElement, ev: PointerEvent) => any} handler
   */
  _removeChoiceEventHandler(el, handler) {
    el.removeEventListener("click", handler);
  }
  
  clearChoices() {
    this.pendingChoices.forEach((choice) => {
      this._removeChoiceEventHandler(choice.el, choice.handler);
    });
    this.pendingChoices = [];
  }

  /**
   * @returns {void}
   */
  clear() {
    this.characters.clear();
  }
}
