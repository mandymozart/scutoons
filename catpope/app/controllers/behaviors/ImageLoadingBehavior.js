/**
 * ImageLoadingBehavior - Image loading controller
 * Handles sequential lazy loading of images
 */

export class ImageLoadingBehavior {
  /**
   * Start loading images
   */
  init(slices) {
    if (slices && slices.length > 0) {
      this.loadImagesSequentially();
    }
  }

  /**
   * Load images one by one in sequence
   */
  loadImagesSequentially() {
    const slices = document.querySelectorAll('.webtoon-slice.lazy');
    let currentIndex = 0;

    const loadNextImage = () => {
      if (currentIndex >= slices.length) {
        console.log('All images loaded');
        return;
      }

      const slice = slices[currentIndex];
      const img = slice.querySelector('.slice-content-img');
      const loadingText = slice.querySelector('.slice-loading-text');
      const placeholderImg = slice.querySelector('.slice-placeholder-img');
      const src = img?.getAttribute('data-src');

      if (!src || !img) {
        currentIndex++;
        loadNextImage();
        return;
      }

      console.log(`Loading image ${currentIndex + 1}/${slices.length}`);

      // Create a new image to preload
      const tempImg = new Image();

      tempImg.onload = () => {
        // Image loaded successfully
        img.src = src;
        img.style.display = 'block';
        if (loadingText) {
          loadingText.style.display = 'none';
        }
        if (placeholderImg) {
          placeholderImg.style.display = 'none';
        }
        slice.classList.remove('lazy');
        slice.classList.add('loaded');

        console.log(`Image ${currentIndex + 1} loaded successfully`);
        currentIndex++;

        // Load next image
        loadNextImage();
      };

      tempImg.onerror = () => {
        // Image failed to load
        console.error(`Failed to load image ${currentIndex + 1}: ${src}`);
        if (loadingText) {
          loadingText.textContent = `Failed to load slice ${currentIndex + 1}`;
          loadingText.style.color = '#e74c3c';
        }
        slice.classList.add('error');
        currentIndex++;

        // Continue with next image even if this one failed
        loadNextImage();
      };

      // Start loading
      tempImg.src = src;
    };

    // Start loading the first image
    loadNextImage();
  }

  /**
   * No cleanup needed for image loading
   */
  cleanup() {
    // Nothing to clean up
  }
}
