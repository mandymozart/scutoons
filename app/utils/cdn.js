import { CDN_SERVER } from "../config.js";

export function asset(path) {
  if (!path || typeof path !== 'string') {
    return path;
  }
  
  if (CDN_SERVER && CDN_SERVER !== '') {
    return `${CDN_SERVER}/${path}`;
  }
  
  return `cdn/root/${path}`;
}
