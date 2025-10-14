/**
 * SongItem component - renders a single song card
 */
export function renderSongItem(song, songNumber) {
  if (song.status === 'published') {
    return `
      <div class="chapter-item song published" onclick="scutoon.navigateTo('song', {song: ${songNumber}})">
        <div class="chapter-image"><img class="chapter-thumbnail" src="${song.thumbnail || 'assets/images/default-thumbnail.jpg'}" alt="${song.title} Thumbnail" loading="lazy"></div>
        <div class="chapter-title">${song.title}</div>
        <div class="chapter-release">${song.release}</div>
        <div class="chapter-number"><img class="chapter-icon" src="../assets/icons/headphones.svg" alt="Song"> #${song.number}</div>
      </div>
    `;
  } else if (song.status === 'coming-soon') {
    return `
      <div class="chapter-item song coming-soon">
        <div class="chapter-image"><img class="chapter-thumbnail" src="${song.thumbnail || 'assets/images/default-thumbnail.jpg'}" alt="${song.title} Thumbnail" loading="lazy"></div>
        <div class="chapter-title">${song.title}</div>
        <div class="chapter-release"><span class="badge">Coming Soon</span><br/>${song.release}</div>
        <div class="chapter-number"><img class="chapter-icon" src="../assets/icons/headphones.svg" alt="Song"> #${song.number}</div>
      </div>
    `;
  }
  return '';
}
