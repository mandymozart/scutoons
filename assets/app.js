/**
 * SCUTOONS Base App, used by all SCUTOONS
 */
class Scutoon {
  toonName;
  lang;
  toon;
  episode;
  season;
  constructor(toonName, lang) {
    this.toonName = toonName;
    this.lang = lang;
  }

  loadToon() {
    this.toon = fetch(`${this.toonName}/${this.lang}/toon.json`)
      .then(response => response.json())
      .then(data => {
        console.log(data);
        return data;
      })
  }

  loadEpisode(number) {
    this.episode = fetch(`${this.toonName}/${this.lang}/chapter${number}.json`)
      .then(response => response.json())
      .then(data => {
        console.log(data);
        return data;
      })
  }

  loadSeason(number) {
    this.season = fetch(`${this.toonName}/${this.lang}/season${number}.json`)
      .then(response => response.json())
      .then(data => {
        console.log(data);
        return data;
      })
  }
}
