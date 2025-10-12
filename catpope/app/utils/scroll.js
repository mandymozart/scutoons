/**
 * Scroll utilities
 */

// Initialize scroll handler for header hide/show
export function initScrollHandler() {
  let lastScrollTop = 0;
  let scrollThreshold = 5; // Minimum scroll amount to trigger
  const header = document.querySelector('header');

  window.addEventListener('scroll', () => {
    const scrollTop = window.pageYOffset || document.documentElement.scrollTop;

    // Skip if scroll change is too small
    if (Math.abs(scrollTop - lastScrollTop) <= scrollThreshold) {
      return;
    }

    if (scrollTop > lastScrollTop && scrollTop > 80) {
      // Scrolling down & past header height
      header?.classList.add('header-hidden');
    } else {
      // Scrolling up
      header?.classList.remove('header-hidden');
    }

    lastScrollTop = scrollTop;
  });
}

// Helper function to scroll to top
export function scrollToTop() {
  window.scrollTo({ top: 0, behavior: 'smooth' });
}
