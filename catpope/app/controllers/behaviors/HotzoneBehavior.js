/**
 * HotzoneBehavior - Hotzone visualization controller
 * Shows interactive audio hotzones with flash animation
 */

export class HotzoneBehavior {
  constructor() {
    this.hotzoneObserver = null;
  }

  /**
   * Initialize hotzone visualizer
   */
  init() {
    // Create IntersectionObserver to detect when hotzones come into view
    this.hotzoneObserver = new IntersectionObserver(
      (entries) => {
        entries.forEach(entry => {
          if (entry.isIntersecting) {
            const hotzone = entry.target;
            
            // Add flash animation
            hotzone.classList.add('flash');
            
            // Remove flash class after animation completes
            setTimeout(() => {
              hotzone.classList.remove('flash');
            }, 500); // Match animation duration
            
            // Keep observing - will flash again on next scroll into view
          }
        });
      },
      {
        threshold: 0.5, // Trigger when 50% visible
        rootMargin: '0px'
      }
    );

    // Observe all hotzones
    const hotzones = document.querySelectorAll('.audio-hotzone');
    hotzones.forEach(hotzone => {
      this.hotzoneObserver.observe(hotzone);
    });

    console.log(`Observing ${hotzones.length} audio hotzones`);
  }

  /**
   * Cleanup hotzones
   */
  destroy() {
    if (this.hotzoneObserver) {
      this.hotzoneObserver.disconnect();
      this.hotzoneObserver = null;
    }
  }
}
