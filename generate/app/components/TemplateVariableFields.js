import { TemplateParser } from '../utils/template.js';

export default class TemplateVariableFields {
    constructor(containerEl, typeSelectEl, toonSelectEl, formEl) {
        this.containerEl = containerEl;
        this.typeSelectEl = typeSelectEl;
        this.toonSelectEl = toonSelectEl;
        this.formEl = formEl;
        
        // Template parser
        this.parser = new TemplateParser();
        
        // State
        this.toons = [];
        this.editions = [];
        this.selectedToon = null;
        this.selectedEdition = null;
        this.templateFields = [];
        
        // Attach listeners
        if (this.typeSelectEl) {
            this.typeSelectEl.addEventListener('change', async () => {
                await this.loadTemplateFields(this.typeSelectEl.value);
                this.render();
            });
        }
        
        if (this.toonSelectEl) {
            this.toonSelectEl.addEventListener('change', () => this.handleToonChange());
        }
        
        // Load toons and initial render
        this.init();
    }

    async init() {
        await this.loadToons();
        await this.loadTemplateFields(this.typeSelectEl ? this.typeSelectEl.value : 'membership');
        this.render();
    }

    async loadTemplateFields(type) {
        // Only load templates for types that have PDFs
        // membership: Certificate of Authenticity
        // promo: QR Print PDF
        // event/xr: No PDFs, no templates needed
        if (type === 'event' || type === 'xr') {
            this.templateFields = [];
            return this.templateFields;
        }

        const templateMap = {
            'membership': 'app/templates/cos.md',
            'promo': 'app/templates/qr.md'
        };

        const templateUrl = templateMap[type];
        if (!templateUrl) {
            this.templateFields = [];
            return this.templateFields;
        }

        this.templateFields = await this.parser.fetchAndParseTemplate(templateUrl);
        
        // Add edition dropdown if membership type and editions are available
        if (type === 'membership' && this.editions.length > 0) {
            this.templateFields.unshift({ 
                id: 'edition_select', 
                label: 'Edition', 
                type: 'select', 
                options: this.editions 
            });
        }

        return this.templateFields;
    }

    renderField(field) {
        const fieldEl = document.createElement('div');
        fieldEl.classList.add('form-group');

        const label = document.createElement('label');
        label.setAttribute('for', field.id);
        label.textContent = field.label + ':';

        let inputEl;
        if (field.type === 'textarea') {
            inputEl = document.createElement('textarea');
            inputEl.id = field.id;
            inputEl.name = field.id;
            inputEl.rows = 3;
            if (field.placeholder) inputEl.placeholder = field.placeholder;
        } else {
            inputEl = document.createElement('input');
            inputEl.type = field.type || 'text';
            inputEl.id = field.id;
            inputEl.name = field.id;
            if (field.placeholder) inputEl.placeholder = field.placeholder;
        }
        
        if (field.required) inputEl.required = true;

        fieldEl.appendChild(label);
        fieldEl.appendChild(inputEl);

        return fieldEl;
    }

    async loadToons() {
        try {
            const response = await fetch('/api/toons/index.php');
            this.toons = await response.json();
            this.populateToonSelect();
        } catch (error) {
            console.error('Error loading toons:', error);
        }
    }

    populateToonSelect() {
        if (!this.toonSelectEl) return;

        // Clear existing options except the first (placeholder)
        while (this.toonSelectEl.options.length > 1) {
            this.toonSelectEl.remove(1);
        }

        // Add toon options
        this.toons.forEach(toon => {
            const option = document.createElement('option');
            option.value = toon.id;
            option.textContent = toon.name;
            option.dataset.redirectUrl = toon.redirectUrl;
            this.toonSelectEl.appendChild(option);
        });

        // Auto-select first toon if available
        if (this.toons.length > 0) {
            this.toonSelectEl.value = this.toons[0].id;
            // Trigger change event to load editions and prefill
            this.handleToonChange();
        }
    }

    async handleToonChange() {
        const selectedToonId = this.toonSelectEl.value;
        
        if (!selectedToonId) {
            this.selectedToon = null;
            this.editions = [];
            this.render();
            return;
        }

        // Find selected toon
        this.selectedToon = this.toons.find(t => t.id === selectedToonId);

        // Prefill redirectUrl
        const redirectUrlInput = document.getElementById('redirectUrl');
        if (redirectUrlInput && this.selectedToon) {
            redirectUrlInput.value = this.selectedToon.redirectUrl;
        }

        // Load editions if membership type
        if (this.typeSelectEl.value === 'membership') {
            await this.loadEditions(selectedToonId);
            await this.loadTemplateFields(this.typeSelectEl.value);
            await this.render();
            
            // Auto-select first edition after render
            if (this.editions.length > 0) {
                const editionSelect = document.getElementById('edition_select');
                if (editionSelect) {
                    editionSelect.value = this.editions[0].id;
                    // Manually trigger the edition selection
                    const selectedOption = editionSelect.options[editionSelect.selectedIndex];
                    if (selectedOption) {
                        this.selectedEdition = {
                            id: selectedOption.value,
                            name: selectedOption.textContent,
                            edition_count: selectedOption.dataset.editionCount,
                            excluded_number: selectedOption.dataset.excludedNumber,
                            creators: selectedOption.dataset.creators
                        };
                        this.prefillEditionFields(this.selectedEdition);
                    }
                }
            }
        } else {
            this.render();
        }
    }

    async loadEditions(toonId) {
        try {
            const response = await fetch(`/api/editions/index.php?toon=${toonId}`);
            const data = await response.json();
            this.editions = Object.values(data); // Convert object to array
        } catch (error) {
            console.error('Error loading editions:', error);
            this.editions = [];
        }
    }

    renderEditionSelect(field) {
        const fieldEl = document.createElement('div');
        fieldEl.classList.add('form-group');

        const label = document.createElement('label');
        label.setAttribute('for', field.id);
        label.textContent = field.label + ':';

        const select = document.createElement('select');
        select.id = field.id;
        select.name = field.id;

        // Add placeholder option
        const placeholderOption = document.createElement('option');
        placeholderOption.value = '';
        placeholderOption.textContent = 'Select an edition';
        select.appendChild(placeholderOption);

        // Add edition options
        field.options.forEach(edition => {
            const option = document.createElement('option');
            option.value = edition.id;
            option.textContent = edition.name;
            option.dataset.editionCount = edition.edition_count;
            option.dataset.excludedNumber = edition.excluded_number;
            option.dataset.creators = edition.creators || '';
            option.dataset.variants = JSON.stringify(edition.variants);
            select.appendChild(option);
        });

        // Handle edition change
        select.addEventListener('change', (e) => {
            const selectedOption = e.target.options[e.target.selectedIndex];
            if (selectedOption.value) {
                // Store selected edition data
                this.selectedEdition = {
                    id: selectedOption.value,
                    name: selectedOption.textContent,
                    edition_count: selectedOption.dataset.editionCount,
                    excluded_number: selectedOption.dataset.excludedNumber,
                    creators: selectedOption.dataset.creators
                };
                
                // Auto-fill all matching template fields with edition data
                this.prefillEditionFields(this.selectedEdition);
            } else {
                this.selectedEdition = null;
            }
        });

        fieldEl.appendChild(label);
        fieldEl.appendChild(select);

        return fieldEl;
    }

    prefillEditionFields(editionData) {
        // Special mapping: edition 'name' -> 'productname' field
        const specialMappings = {
            'name': 'productname'
        };

        // Iterate through all edition data properties
        Object.keys(editionData).forEach(key => {
            // Check if there's a special mapping for this key
            const fieldId = specialMappings[key] || key;
            
            // Find the corresponding form field
            const element = document.getElementById(fieldId);
            
            // Prefill if element exists and has a value
            if (element && editionData[key] != null) {
                // Skip complex objects/arrays (like variants)
                if (typeof editionData[key] !== 'object') {
                    element.value = editionData[key];
                }
            }
        });
    }

    async render() {
        // Clear container
        this.containerEl.innerHTML = '';

        const type = this.typeSelectEl ? this.typeSelectEl.value : 'membership';
        const fields = this.templateFields;

        if (fields.length > 0) {
            const heading = document.createElement('h3');
            heading.textContent = 'Certificate Fields';
            heading.style.marginTop = '1rem';
            this.containerEl.appendChild(heading);

            fields.forEach(field => {
                if (field.type === 'select') {
                    this.containerEl.appendChild(this.renderEditionSelect(field));
                } else {
                    this.containerEl.appendChild(this.renderField(field));
                }
            });

            // Add hidden fields for edition data if edition is selected
            if (this.selectedEdition && this.typeSelectEl.value === 'membership') {
                const hiddenFields = [
                    { name: 'edition_count', value: this.selectedEdition.edition_count },
                    { name: 'excluded_number', value: this.selectedEdition.excluded_number }
                ];

                hiddenFields.forEach(field => {
                    const input = document.createElement('input');
                    input.type = 'hidden';
                    input.name = field.name;
                    input.value = field.value;
                    this.containerEl.appendChild(input);
                });
            }
        }
    }
}