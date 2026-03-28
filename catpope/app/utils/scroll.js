/**
 * Scroll utilities
 */

/**
 * Scroll to top of page
 */
export function scrollToTop() {
  window.scrollTo({ top: 0, behavior: 'smooth' });
}

/**
 * Scroll to a specific label/element
 * @param {string} label - Label to scroll to
 */
export function scrollToLabel(label) {
  const element = document.querySelector(`[data-label="${label}"]`);
  
  if (element) {
    element.scrollIntoView({
      behavior: 'smooth',
      block: 'start'
    });
  } else {
    console.warn(`Label not found for scrolling: ${label}`);
    scrollToTop();
  }
}

/**
 * Initialize scroll handler for header hide/show behavior
 * Called by Header component after rendering
 */
export function initHeaderScrollHandler() {
  let lastScrollTop = 0;
  let scrollThreshold = 5;
  const header = document.querySelector('header');

  window.addEventListener('scroll', () => {
    const scrollTop = window.pageYOffset || document.documentElement.scrollTop;

    if (Math.abs(scrollTop - lastScrollTop) <= scrollThreshold) {
      return;
    }

    if (scrollTop > lastScrollTop && scrollTop > 80) {
      header?.classList.add('header-hidden');
    } else {
      header?.classList.remove('header-hidden');
    }

    lastScrollTop = scrollTop;
  });
}
