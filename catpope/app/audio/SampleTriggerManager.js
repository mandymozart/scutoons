/**
 * SampleTriggerManager - Event-based audio sample playback
 * Handles loading, caching, and triggering audio samples based on user interaction
 */

export class SampleTriggerManager {
  constructor(audioManager) {
    this.audioManager = audioManager;
    this.sampleLibrary = new Map(); // Map<sampleId, Howl>
    this.activeTriggers = new Map(); // Map<sliceElement, triggerCleanup[]>
    this.intersectionObserver = null;
  }

  /**
   * Load sample library from config
   * @param {Array} sampleLibrary - Array of {id, path} objects
   */
  async loadSampleLibrary(sampleLibrary) {
    console.log('Loading sample library...');

    if (!sampleLibrary || sampleLibrary.length === 0) {
      console.warn('No samples to load');
      return;
    }

    // Load all samples
    const loadPromises = sampleLibrary.map(sampleConfig => 
      this.loadSample(sampleConfig)
    );

    await Promise.all(loadPromises);
    console.log(`Loaded ${this.sampleLibrary.size} samples`);
  }

  /**
   * Load a single sample
   * @param {Object} sampleConfig - Sample configuration {id, path}
   */
  loadSample(sampleConfig) {
    return new Promise((resolve, reject) => {
      const howl = new Howl({
        src: [sampleConfig.path],
        preload: true,
        html5: false,
        onload: () => {
          console.log(`Sample loaded: ${sampleConfig.id}`);
          resolve();
        },
        onloaderror: (id, error) => {
          console.error(`Failed to load sample: ${sampleConfig.id}`, error);
          reject(error);
        }
      });

      this.sampleLibrary.set(sampleConfig.id, howl);
    });
  }

  /**
   * Register triggers for a slice element
   * @param {HTMLElement} sliceElement - The slice DOM element
   * @param {Array} sampleConfigs - Array of sample trigger configurations
   */
  registerSliceTriggers(sliceElement, sampleConfigs) {
    if (!sampleConfigs || sampleConfigs.length === 0) {
      return;
    }

    const cleanupFunctions = [];

    sampleConfigs.forEach(config => {
      const cleanup = this.registerTrigger(sliceElement, config);
      if (cleanup) {
        cleanupFunctions.push(cleanup);
      }
    });

    // Store cleanup functions for later
    this.activeTriggers.set(sliceElement, cleanupFunctions);
  }

  /**
   * Register a single trigger
   * @param {HTMLElement} sliceElement - The slice DOM element
   * @param {Object} config - Sample trigger configuration
   * @returns {Function|null} Cleanup function
   */
  registerTrigger(sliceElement, config) {
    const triggerType = config.trigger?.type || 'scrollInView';

    switch (triggerType) {
      case 'scrollInView':
        return this.registerScrollInViewTrigger(sliceElement, config);
      case 'click':
        return this.registerClickTrigger(sliceElement, config);
      case 'hover':
        return this.registerHoverTrigger(sliceElement, config);
      default:
        console.warn(`Unknown trigger type: ${triggerType}`);
        return null;
    }
  }

  /**
   * Register scrollInView trigger using IntersectionObserver
   */
  registerScrollInViewTrigger(sliceElement, config) {
    // Create observer if it doesn't exist
    if (!this.intersectionObserver) {
      this.intersectionObserver = new IntersectionObserver(
        (entries) => {
          entries.forEach(entry => {
            if (entry.isIntersecting) {
              // Get the config from the element's data
              const sampleConfig = entry.target._sampleConfig;
              if (sampleConfig) {
                this.trigger(sampleConfig);
              }
            }
          });
        },
        {
          threshold: 0.5, // Trigger when 50% visible
          rootMargin: '0px'
        }
      );
    }

    // Store config on element for callback
    sliceElement._sampleConfig = config;

    // Observe the element
    this.intersectionObserver.observe(sliceElement);

    // Return cleanup function
    return () => {
      this.intersectionObserver.unobserve(sliceElement);
      delete sliceElement._sampleConfig;
    };
  }

  /**
   * Register click trigger
   */
  registerClickTrigger(sliceElement, config) {
    const clickHandler = (event) => {
      // Check if click is within rect bounds (if specified)
      if (config.trigger.rect) {
        if (!this.isWithinRect(event, sliceElement, config.trigger.rect)) {
          return;
        }
      }

      this.trigger(config);
    };

    sliceElement.addEventListener('click', clickHandler);

    // Return cleanup function
    return () => {
      sliceElement.removeEventListener('click', clickHandler);
    };
  }

  /**
   * Register hover trigger
   */
  registerHoverTrigger(sliceElement, config) {
    const hoverHandler = (event) => {
      // Check if hover is within rect bounds (if specified)
      if (config.trigger.rect) {
        if (!this.isWithinRect(event, sliceElement, config.trigger.rect)) {
          return;
        }
      }

      this.trigger(config);
    };

    sliceElement.addEventListener('mouseenter', hoverHandler);

    // Return cleanup function
    return () => {
      sliceElement.removeEventListener('mouseenter', hoverHandler);
    };
  }

  /**
   * Check if event is within specified rect
   */
  isWithinRect(event, element, rect) {
    const bounds = element.getBoundingClientRect();
    const x = ((event.clientX - bounds.left) / bounds.width) * 100;
    const y = ((event.clientY - bounds.top) / bounds.height) * 100;

    return (
      x >= rect.x &&
      x <= rect.x + rect.width &&
      y >= rect.y &&
      y <= rect.y + rect.height
    );
  }

  /**
   * Trigger a sample with given configuration
   * @param {Object} config - Sample trigger configuration
   */
  trigger(config) {
    const howl = this.sampleLibrary.get(config.id);
    if (!howl) {
      console.warn(`Sample not found: ${config.id}`);
      return;
    }

    // Handle mono mode - stop if already playing
    if (config.trigger?.mode === 'mono' && howl.playing()) {
      howl.stop();
    }

    // Handle gating - don't play if already playing
    if (config.trigger?.gate && howl.playing()) {
      return;
    }

    // Handle exclusive mode - mute other audio
    if (config.trigger?.exclusive) {
      // TODO: Implement exclusive mode (mute other samples/music)
      console.log('Exclusive mode not yet implemented');
    }

    // Set sample-specific volume
    if (config.volume !== undefined) {
      howl.volume(config.volume);
    }

    // Set loop
    if (config.loop !== undefined) {
      howl.loop(config.loop);
    }

    // Apply delay if specified
    const delay = config.delay || 0;
    
    if (delay > 0) {
      setTimeout(() => {
        howl.play();
      }, delay);
    } else {
      howl.play();
    }

    console.log(`Triggered sample: ${config.id}`);
  }

  /**
   * Cleanup all triggers
   */
  cleanup() {
    console.log('Cleaning up sample triggers...');

    // Run all cleanup functions
    this.activeTriggers.forEach((cleanupFunctions) => {
      cleanupFunctions.forEach(cleanup => cleanup());
    });

    this.activeTriggers.clear();

    // Disconnect observer
    if (this.intersectionObserver) {
      this.intersectionObserver.disconnect();
      this.intersectionObserver = null;
    }

    // Stop and unload all samples
    this.sampleLibrary.forEach((howl) => {
      howl.unload();
    });

    this.sampleLibrary.clear();

    console.log('Sample triggers cleaned up');
  }
}
