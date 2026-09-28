import { CDN_SERVER } from "../config.ts";
import { sanitizeUrl } from "./template.js";

export function asset(path: string) : string {
  if (!path || typeof path !== 'string') {
    return '';
  }
  
  const safePath = sanitizeUrl(path);
  if (!safePath) {
    return '';
  }
  
  return `${CDN_SERVER}/${safePath.replace(/^\/+/, '')}`;
}
