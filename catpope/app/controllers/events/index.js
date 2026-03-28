export * from "./showMessage.js"
export * from "./addChoice.js"
export * from "./showChoices.js"
export * from "./joinScene.js"
export * from "./leaveScene.js"


  /**
   * Show choices UI (prepare for choices)
   * @param {VNShowChoicesEvent} event
   * @returns {Promise<void>}
   */
  async showChoices(scutoon, event) {
    console.log('showChoices');
    
    if (!this.choicesContainer) {
      this.buildUI();
    }

    // Clear previous choices
    this.choicesContainer.innerHTML = '';
    this.pendingChoices = [];
    
    // Show choices container
    this.choicesContainer.style.display = 'flex';
  }

  /**
   * Add a choice button
   * @param {VNAddChoiceEvent} event
   * @returns {Promise<void>}
   */
  async addChoice({ params }) {
    const { label, onSelect } = params;
    console.log(`addChoice: ${label}`);
    
    if (!this.choicesContainer) {
      this.buildUI();
    }

    // Render choice button using component
    const choiceIndex = this.pendingChoices.length;
    const choiceHtml = renderChoice(label, choiceIndex);
    this.choicesContainer.insertAdjacentHTML('beforeend', choiceHtml);
    
    // Get the newly added button
    const button = /** @type {HTMLElement} */ (this.choicesContainer.lastElementChild);
    
    // Store choice handler
    const choiceHandler = () => {
      // Handle variable modification
      if ('set' in onSelect) {
        scutoon.gameStore.set(onSelect.set.variable, onSelect.set.value);
        console.log(`Set ${onSelect.set.variable} = ${onSelect.set.value}`);
      } else if ('add' in onSelect) {
        const current = this.scutoon.gameStore.get(onSelect.add.variable);
        const currentNum = typeof current === 'number' ? current : 0;
        const newValue = currentNum + onSelect.add.value;
        scutoon.gameStore.set(onSelect.add.variable, newValue);
        console.log(`Add ${onSelect.add.variable}: ${currentNum} + ${onSelect.add.value} = ${newValue}`);
      }
      
      // Hide choices after selection
      this.choicesContainer.style.display = 'none';
    };

    scutoons.visualNovelStore.addChoice({ button, handler: choiceHandler });

    // Wait for THIS choice to be clicked
    return new Promise(resolve => {
      button.addEventListener('click', () => {
        choiceHandler();
        scutoons.visualNovelStore.clearChoices();
        resolve();
      });
    });
  }

  /**
   * Add characters to scene
   * @param {VNJoinSceneEvent} event
   * @returns {Promise<void>}
   */
  async joinScene({ params }) {
    const { characters } = params;
    console.log(`joinScene: ${characters.map(c => c.name).join(', ')}`);
    
    if (!this.charactersContainer) {
      this.buildUI();
    }

    for (const charRef of characters) {
      const charData = this.characters.get(charRef.name);
      if (!charData) {
        console.warn(`Character not found: ${charRef.name}`);
        continue;
      }

      // Render character using component
      const characterHtml = renderCharacter(charData);
      this.charactersContainer.insertAdjacentHTML('beforeend', characterHtml);
      
      // Get the newly added character element for positioning
      const charEl = /** @type {HTMLElement} */ (this.charactersContainer.lastElementChild);
      
      // Position character
      if (charRef.position) {
        charEl.style.order = String(charRef.position.order || 0);
        charEl.dataset.layer = charRef.position.relativeToMessageBox || 'front';
      }

      this.activeCharacters.add(charRef.name);
    }
  }

  /**
   * Remove characters from scene
   * @param {VNLeaveSceneEvent} event
   * @returns {Promise<void>}
   */
  async leaveScene({ params }) {
    const { characters } = params;
    console.log(`leaveScene: ${characters.map(c => c.name).join(', ')}`);
    
    for (const charRef of characters) {
      const charEl = this.charactersContainer?.querySelector(`[data-character="${charRef.name}"]`);
      if (charEl) {
        charEl.remove();
        this.activeCharacters.delete(charRef.name);
      }
    }
  }

  /**
   * Build visual novel UI
   */
  buildUI() {
    const contentEl = this.scutoon.contentEl;
    if (!contentEl) return;

    const vnUIHtml = `
      <div class="vn-characters"></div>
      ${renderMessageBox('', '', false)}
      <div class="vn-choices"></div>
    `;
    
    contentEl.insertAdjacentHTML('beforeend', vnUIHtml);
    
    this.charactersContainer = /** @type {HTMLElement} */ (contentEl.querySelector('.vn-characters'));
    this.messageBox = /** @type {HTMLElement} */ (contentEl.querySelector('.vn-message-box'));
    this.choicesContainer = /** @type {HTMLElement} */ (contentEl.querySelector('.vn-choices'));
    
    this.messageBox.style.display = 'none';
    this.choicesContainer.style.display = 'none';

    console.log('Visual Novel UI built');
  }

  /**
   * Cleanup visual novel UI
   */
  cleanup() {
    if (this.messageBox) {
      this.messageBox.remove();
      this.messageBox = null;
    }
    if (this.choicesContainer) {
      this.choicesContainer.remove();
      this.choicesContainer = null;
    }
    if (this.charactersContainer) {
      this.charactersContainer.remove();
      this.charactersContainer = null;
    }
    
    this.activeCharacters.clear();
    this.pendingChoices = [];
    
    console.log('Visual Novel UI cleaned up');
  }
}
