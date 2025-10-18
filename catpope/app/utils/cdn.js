import { TOON_ID, CDN_SERVER } from "../config.js";
import { sanitizeUrl, escapeAttr } from "./template.js";

export function asset(path) {
  if (!path || typeof path !== 'string') {
    return '';
  }
  
  const safePath = sanitizeUrl(path);
  if (!safePath) {
    return '';
  }
  
  let url;
  if (CDN_SERVER && CDN_SERVER !== '') {
    url = `${CDN_SERVER}/${TOON_ID}/${safePath}`;
  } else {
    url = `../../cdn/root/${TOON_ID}/${safePath}`;
  }
  
  return escapeAttr(url);
}
