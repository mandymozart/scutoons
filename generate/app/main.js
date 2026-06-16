import GenerateView from './views/generate.js';
import RegenerateView from './views/regenerate.js';
import BatchView from './views/batch.js';
import TemplateVariableFields from './components/TemplateVariableFields.js';

const App = {
	init() {
		// Initialize template variable fields
		const templateVarsContainer = document.querySelector('.template-variables');
		const typeSelect = document.getElementById('type');
		const toonSelect = document.getElementById('toon');
		const formEl = document.getElementById('qrForm');
		
		if (templateVarsContainer && typeSelect && toonSelect && formEl) {
			new TemplateVariableFields(templateVarsContainer, typeSelect, toonSelect, formEl);
		}
		
		// Initialize views
		new GenerateView();
		new RegenerateView();
		new BatchView();
		
		// Setup mode switching
		this.setupModeSwitching();
		
		console.log('QR Generator App initialized');
	},
	
	setupModeSwitching() {
		const modeOptions = document.querySelectorAll('.mode-option');
		const modePanels = document.querySelectorAll('.mode-panel');
		
		modeOptions.forEach(option => {
			option.addEventListener('click', () => {
				const targetMode = option.getAttribute('data-mode');
				
				// Update active mode option
				modeOptions.forEach(opt => opt.classList.remove('active'));
				option.classList.add('active');
				
				// Show corresponding panel
				modePanels.forEach(panel => {
					if (panel.id === targetMode + 'Mode') {
						panel.classList.add('active');
					} else {
						panel.classList.remove('active');
					}
				});
			});
		});
	}
};

// Start app when DOM is ready
if (document.readyState === 'loading') {
	document.addEventListener('DOMContentLoaded', () => App.init());
} else {
	App.init();
}