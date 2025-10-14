import { Scutoon } from './Scutoon.js';
import { parseURL, renderView } from './utils/navigation.js';
import { scrollToTop } from './utils/scroll.js';

const scutoon = new Scutoon('de');
window.scutoon = scutoon;

async function handleNavigation() {
  const navParams = parseURL();
  await scutoon.init();
  await renderView(scutoon, navParams);
  scrollToTop();
}

document.addEventListener('DOMContentLoaded', () => handleNavigation());
window.addEventListener('popstate', () => handleNavigation());
