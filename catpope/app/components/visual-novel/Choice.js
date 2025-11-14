/**
 * @typedef {import('../../types.js').IScutoon} IScutoon
 * @typedef {import('../../types.js').VNAddChoiceEvent} VNAddChoiceEvent
 */
import { escapeHtml } from "../../utils/template.js";
import { hideChoices } from "./Choices.js";

/**
 * @param {string} label
 * @param {number} index
 * @returns {string}
 */
function renderChoice(label, index) {
  return `
    <button class="vn-choice" data-choice-index="${index}">
      ${escapeHtml(label)}
    </button>
  `;
}

/**
 * @param {IScutoon} scutoon
 * @param {HTMLElement} choicesEl
 * @param {VNAddChoiceEvent} event
 * @returns {Promise<void>}
 */
export async function handleAddChoice(scutoon, choicesEl, event) {
  const { label, onSelect } = event.params;
  console.log(`addChoice: ${label}`);

  const choiceIndex = scutoon.visualNovelStore.pendingChoices.length;
  const choiceHtml = renderChoice(label, choiceIndex);
  choicesEl.append(choiceHtml);

  const button = /** @type {HTMLElement} */ (choicesEl.lastElementChild);

  // TODO cleaner on select handling with bigger complexity.
  // because variable functions can also be used in different events
  const choiceHandler = () => {
    // Handle variable modification
    if ("set" in onSelect) {
      scutoon.gameStore.set(onSelect.set.variable, onSelect.set.value);
      console.log(`Set ${onSelect.set.variable} = ${onSelect.set.value}`);
    } else if ("add" in onSelect) {
      const current = scutoon.gameStore.get(onSelect.add.variable);
      const currentNum = typeof current === "number" ? current : 0;
      const newValue = currentNum + onSelect.add.value;
      scutoon.gameStore.set(onSelect.add.variable, newValue);
      console.log(
        `Add ${onSelect.add.variable}: ${currentNum} + ${onSelect.add.value} = ${newValue}`
      );
    }

    hideChoices(choicesEl);
  };

  scutoon.visualNovelStore.addChoice({ button, handler: choiceHandler });

  // Wait for THIS choice to be clicked
  // THIS IS BLOCKING THE UI  wait for any choice in the show choices
  return new Promise((resolve) => {
    button.addEventListener("click", () => {
      choiceHandler();
      scutoon.visualNovelStore.clearChoices();
      resolve();
    });
  });
}
