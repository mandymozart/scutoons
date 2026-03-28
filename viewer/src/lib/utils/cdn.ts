import { CDN_SERVER } from "../config.ts";
import { sanitizeUrl } from "./template.js";
const isProd = import.meta.env.PROD;

export function asset(path: string) : string {
  if (!path || typeof path !== 'string') {
    return '';
  }
  
  const safePath = sanitizeUrl(path);
  if (!safePath) {
    return '';
  }
  
  if (CDN_SERVER && isProd) {
    return `${CDN_SERVER}/${safePath}`;
  } else {
    return `http://localhost:8000/${safePath}`;
  }
}
