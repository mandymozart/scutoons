/**
 * API utilities for loading content
 */

export async function loadToon(toonName, lang) {
  return fetch(`${toonName}/${lang}/toon.json`)
    .then(response => response.json())
    .then(data => {
      console.log('Toon data:', data);
      return data;
    });
}

export async function loadChapter(toonName, lang, chapterNumber) {
  return fetch(`${toonName}/${lang}/chapter${chapterNumber}.json`)
    .then(response => response.json())
    .then(data => {
      console.log('Chapter data:', data);
      return data;
    });
}

export async function loadSeason(toonName, lang, number) {
  return fetch(`${toonName}/${lang}/season${number}.json`)
    .then(response => response.json())
    .then(data => {
      console.log('Season data:', data);
      return data;
    });
}

export async function loadSong(toonName, lang, songNumber) {
  return fetch(`${toonName}/${lang}/song${songNumber}.json`)
    .then(response => response.json())
    .then(data => {
      console.log('Song data:', data);
      return data;
    });
}

export async function loadCredits(toonName, lang) {
  return fetch(`${toonName}/${lang}/credits.json`)
    .then(response => response.json());
}

// Load songs metadata for player
export async function loadSongs(toonName, lang) {
  const songPromises = [];
  for (let i = 1; i <= 6; i++) {
    songPromises.push(
      fetch(`${toonName}/${lang}/song${i}.json`)
        .then(response => response.json())
        .then(data => ({ id: i, ...data }))
    );
  }
  return Promise.all(songPromises);
}
