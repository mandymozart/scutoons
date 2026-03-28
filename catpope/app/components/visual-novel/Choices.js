/**
 * @typedef {import('../../types.js').VNShowChoicesEvent} VNShowChoicesEvent
 * @typedef {import('../../types.js').IScutoon} IScutoon
 */

/**
 * @returns {string}
 */
export function renderChoices() {

  return `
    <div class="vn-choices">
      <!-- Choices will be dynamically added here -->
    </div>
  `;
}

/**
 * @param {IScutoon} scutoon
 * @param {HTMLElement} el
 * @param {VNShowChoicesEvent} event
 * @returns {Promise<void>}
 */
export async function handleShowChoices(scutoon, el, event) {
  console.log("showChoices");
  el.classList.add("active");
  return new Promise((resolve) => {
    resolve();
  });
}

/**
 * @param {HTMLElement} el
 */
export function hideChoices(el) {
  el.classList.remove("active");
}
