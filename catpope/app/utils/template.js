/**
 * Template utilities for safe HTML rendering
 */

/**
 * Escapes HTML special characters to prevent XSS attacks
 * @param {string} text - The text to escape
 * @returns {string} - Escaped text safe for HTML insertion
 */
export function escapeHtml(text) {
  if (text === null || text === undefined) {
    return '';
  }
  
  const str = String(text);
  const htmlEscapeMap = {
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#x27;',
    '/': '&#x2F;'
  };
  
  return str.replace(/[&<>"'/]/g, (char) => htmlEscapeMap[char]);
}

/**
 * Sanitizes URLs to prevent javascript: and data: URIs
 * @param {string} url - The URL to sanitize
 * @returns {string} - Safe URL or empty string if dangerous
 */
export function sanitizeUrl(url) {
  if (!url) return '';
  
  const urlStr = String(url).trim().toLowerCase();
  
  // Block dangerous protocols
  const dangerousProtocols = ['javascript:', 'data:', 'vbscript:', 'file:'];
  for (const protocol of dangerousProtocols) {
    if (urlStr.startsWith(protocol)) {
      console.warn('Blocked dangerous URL protocol:', url);
      return '';
    }
  }
  
  return url;
}

/**
 * Escapes HTML attribute values
 * @param {string} value - The attribute value to escape
 * @returns {string} - Escaped attribute value
 */
export function escapeAttr(value) {
  if (value === null || value === undefined) {
    return '';
  }
  
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#x27;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

/**
 * Joins and escapes an array of strings
 * @param {Array} arr - Array of strings
 * @param {string} separator - Separator (default: ', ')
 * @returns {string} - Escaped joined string
 */
export function escapeAndJoin(arr, separator = ', ') {
  if (!Array.isArray(arr)) return '';
  return arr.map(item => escapeHtml(item)).join(separator);
}

/**
 * Template helper to replace placeholders with escaped data
 * @param {string} template - Template string with {{key}} placeholders
 * @param {Object} data - Data object
 * @returns {string} - Rendered template with escaped values
 */
export function renderTemplate(template, data) {
  return template.replace(/\{\{(\w+)\}\}/g, (match, key) => {
    if (key === 'authors' && Array.isArray(data[key])) {
      return escapeAndJoin(data[key]);
    }
    return escapeHtml(data[key]);
  });
}

/**
 * Validates and sanitizes a number parameter
 * @param {any} value - Value to validate
 * @param {number} defaultValue - Default value if invalid
 * @param {number} min - Minimum allowed value
 * @param {number} max - Maximum allowed value
 * @returns {number} - Validated number
 */
export function sanitizeNumber(value, defaultValue = 1, min = 1, max = 999) {
  const num = parseInt(value, 10);
  if (isNaN(num) || num < min || num > max) {
    return defaultValue;
  }
  return num;
}
