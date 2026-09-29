import TemplateVariableFields from './components/TemplateVariableFields.js';
import { getToken } from './utils/auth.js';
import AuthView from './views/AuthView.js';
import BatchView from './views/batch.js';
import StatsView from './views/stats.js';
import GenerateView from './views/generate.js';
import RegenerateView from './views/regenerate.js';

const App = {
  async init() {
    const authView = new AuthView();
    await authView.require();

    const appEl = document.getElementById('app');
    if (appEl) appEl.removeAttribute('hidden');

    // Patch window.fetch so every relative request carries the token automatically
    const _fetch = window.fetch.bind(window);
    window.fetch = (url, options = {}) => {
      if (typeof url === 'string' && !url.match(/^https?:\/\//)) {
        options = {
          ...options,
          headers: { 'X-Generate-Token': getToken(), ...options.headers },
        };
      }
      return _fetch(url, options);
    };

    // Initialize template variable fields
    const templateVarsContainer = document.querySelector('.template-variables');
    const typeSelect = document.getElementById('type');
    const toonSelect = document.getElementById('toon');
    const formEl = document.getElementById('qrForm');

    if (templateVarsContainer && typeSelect && toonSelect && formEl) {
      new TemplateVariableFields(
        templateVarsContainer,
        typeSelect,
        toonSelect,
        formEl,
      );
    }

    // Initialize views
    new GenerateView();
    new RegenerateView();
    try {
      new BatchView();
      new StatsView();
    } catch (err) {
      console.error('[main] BatchView failed to initialise:', err);
      const el = document.getElementById('batchMode');
      if (el)
        el.innerHTML = `<p class="error">Batch view error: ${err.message}</p>`;
    }

    // Setup mode switching
    this.setupModeSwitching();

    console.log('QR Generator App initialized');
  },

  setupModeSwitching() {
    const modeOptions = document.querySelectorAll('.mode-option');
    const modePanels = document.querySelectorAll('.mode-panel');

    modeOptions.forEach((option) => {
      option.addEventListener('click', () => {
        const targetMode = option.getAttribute('data-mode');

        // Update active mode option
        modeOptions.forEach((opt) => opt.classList.remove('active'));
        option.classList.add('active');

        // Show corresponding panel
        modePanels.forEach((panel) => {
          if (panel.id === targetMode + 'Mode') {
            panel.classList.add('active');
          } else {
            panel.classList.remove('active');
          }
        });
      });
    });
  },
};

// Start app when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', () => App.init());
} else {
  App.init();
}
