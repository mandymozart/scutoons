import GenerateView from './views/generate.js';
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
		
		// Initialize generate view
		new GenerateView();
		
		console.log('QR Generator App initialized');
	}
};

// Start app when DOM is ready
if (document.readyState === 'loading') {
	document.addEventListener('DOMContentLoaded', () => App.init());
} else {
	App.init();
}