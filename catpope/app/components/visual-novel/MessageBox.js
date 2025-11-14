/**
 * @typedef {import("../../types.js").IScutoon} IScutoon
 * @typedef {import("../../types.js").Character} Character
 * @typedef {import("../../models/visual-novel.js").VNShowMessageEvent} VNShowMessageEvent
 */
import { escapeHtml } from "../../utils/template.js";

// Singleton
const selector = "vn-message-box";

/**
 * @param {Character} character
 * @param {string} text
 * @param {boolean} showContinue
 * @returns {string}
 */
export function renderMessageBox(
  character = null,
  text = "...",
  showContinue = false
) {
  return `
    <div id="${selector}">
      ${
        character
          ? `<div class="vn-character-name">${escapeHtml(character.name)}</div>`
          : ""
      }
      <div class="vn-message-text">${escapeHtml(text)}</div>
      ${showContinue ? `<div class="vn-continue-hint">...</div>` : ""}
    </div>
  `;
}

/**
 * @param {IScutoon} scutoon
 * @param {HTMLElement} el
 * @param {VNShowMessageEvent} event
 * @returns {Promise<void>}
 */
export function handleShowMessage(scutoon, el, event) {
  const { text, characterName, wait = true } = event.params;
  console.log(`showMessage: ${characterName}: ${text}`);

  const html = renderMessageBox(
    scutoon.visualNovelStore.characters[characterName],
    text,
    wait
  );
  el.outerHTML = html;
  el = /** @type {HTMLElement} */ (
    scutoon.contentEl.querySelector(`#${selector}`)
  );
  el.classList.add("active");

  if (wait) {
    // Wait for user to click message box
    return new Promise((resolve) => {
      const clickHandler = () => {
        el.removeEventListener("click", clickHandler);
        resolve();
      };
      el.addEventListener("click", clickHandler);
    });
  }
}

/**
 * @param {IScutoon} scutoon
 */
export function hideMessageBox(scutoon) {
  const el = scutoon.contentEl.querySelector(`#${selector}`);
  if (el) {
    el.classList.remove("active");
  }
}
