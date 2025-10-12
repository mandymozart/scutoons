/**
 * Template utilities
 */

// Template helper to replace placeholders with data
export function renderTemplate(template, data) {
  return template.replace(/\{\{(\w+)\}\}/g, (match, key) => {
    if (key === 'authors' && Array.isArray(data[key])) {
      return data[key].join(', ');
    }
    return data[key] || '';
  });
}
