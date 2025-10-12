/**
 * Image loading utilities for webtoon slices
 */

// Load images one by one in sequence
export function loadImagesSequentially() {
  const slices = document.querySelectorAll('.webtoon-slice.lazy');
  let currentIndex = 0;

  const loadNextImage = () => {
    if (currentIndex >= slices.length) {
      console.log('All images loaded');
      return;
    }

    const slice = slices[currentIndex];
    const img = slice.querySelector('img');
    const placeholder = slice.querySelector('.slice-placeholder');
    const src = img.getAttribute('data-src');

    if (!src) {
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
      if (placeholder) {
        placeholder.style.display = 'none';
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
      if (placeholder) {
        placeholder.textContent = `Failed to load image ${currentIndex + 1}`;
        placeholder.style.color = '#e74c3c';
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
