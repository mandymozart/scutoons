/**
 * Music player functionality
 */

// Initialize player functionality
export function initPlayer(songList) {
  let currentId = 0;
  let isPlaying = false;
  let isLoop = true;
  let isShuffle = false;
  let currentAudio = "music1";
  let timer = null;
  let loopOne = false;

  const currentTimeIndicator = document.querySelector(".music-time__current");
  const leftTimeIndicator = document.querySelector(".music-time__last");
  const progressBar = document.getElementById("length");
  const playBtn = document.querySelector(".play");
  const title = document.querySelector(".music-player__title");
  const author = document.querySelector(".music-player__author");

  const loopBtn = document.getElementById("loop");
  const shuffleBtn = document.getElementById("shuffle");
  const prevBtn = document.getElementById("prev");
  const nextBtn = document.getElementById("next");
  const progressDiv = document.getElementById("progress");

  const play = (e) => {
    if (!isPlaying) {
      e.target.src = "https://snowleo208.github.io/100-Days-of-Code/7.%20Music%20Player/img/pause.svg";
      e.target.alt = "Pause";
      isPlaying = true;
      document.getElementById(currentAudio).play();
      showTime();
    } else {
      e.target.src = "https://snowleo208.github.io/100-Days-of-Code/7.%20Music%20Player/img/play.svg";
      e.target.alt = "Play";
      document.getElementById(currentAudio).pause();
      isPlaying = false;
      clearInterval(timer);
    }
  };

  const changeBar = () => {
    const audio = document.getElementById(currentAudio);
    const percentage = (audio.currentTime / audio.duration).toFixed(3);
    progressBar.style.transition = "";

    const minute = Math.floor(audio.currentTime / 60);
    const second = Math.floor(audio.currentTime % 60);
    const leftTime = audio.duration - audio.currentTime;
    currentTimeIndicator.innerHTML = ("0" + minute).substr(-2) + ":" + ("0" + second).substr(-2);

    const leftMinute = Math.floor(leftTime / 60);
    const leftSecond = Math.floor(leftTime % 60);
    leftTimeIndicator.innerHTML = ("0" + leftMinute).substr(-2) + ":" + ("0" + leftSecond).substr(-2);

    progressBar.style.width = percentage * 100 + "%";
  };

  const showTime = () => {
    timer = setInterval(() => changeBar(), 500);
  };

  const nextMusic = (mode) => {
    playBtn.src = "https://snowleo208.github.io/100-Days-of-Code/7.%20Music%20Player/img/play.svg";
    playBtn.alt = "Play";
    document.getElementById(currentAudio).pause();
    isPlaying = false;
    clearInterval(timer);

    if (mode === "next") {
      currentId = currentId + 1 > songList.length - 1 ? 0 : currentId + 1;
      init();
    } else {
      currentId = currentId - 1 < 0 ? songList.length - 1 : currentId - 1;
      init();
    }
  };

  const shuffle = (e) => {
    isShuffle = !isShuffle;
    if (isShuffle) {
      e.target.parentNode.classList.add("is-loop");
    } else {
      e.target.parentNode.classList.remove("is-loop");
    }
  };

  const backward = () => {
    const audio = document.getElementById(currentAudio);
    audio.currentTime -= 5;
    if (!isPlaying) {
      changeBar();
    }
  };

  const forward = () => {
    const audio = document.getElementById(currentAudio);
    audio.currentTime += 5;
    if (!isPlaying) {
      changeBar();
    }
  };

  const stopMusic = () => {
    playBtn.src = "https://snowleo208.github.io/100-Days-of-Code/7.%20Music%20Player/img/play.svg";
    playBtn.alt = "Play";
    isPlaying = false;
  };

  const goToNextMusic = () => {
    let newId = currentId;
    while (isShuffle && !loopOne && newId === currentId) {
      newId = Math.floor(Math.random() * Math.floor(songList.length - 1));
    }

    if (!isShuffle && !loopOne) {
      currentId = currentId + 1 > songList.length - 1 ? 0 : currentId + 1;
    }
    if (!isShuffle && loopOne) {
      currentId = currentId;
    }

    if (isShuffle) {
      currentId = newId;
    }
    init();
    document.getElementById(currentAudio).play();
  };

  const loop = (e) => {
    const audio = document.getElementById(currentAudio);

    if (!isLoop && !loopOne) {
      isLoop = true;
      loopOne = false;
      e.target.parentNode.classList.add("is-loop");
      e.target.src = "https://snowleo208.github.io/100-Days-of-Code/7.%20Music%20Player/img/loop.svg";
      audio.loop = false;
      audio.onended = e => goToNextMusic();
    } else if (isLoop && !loopOne) {
      isLoop = true;
      loopOne = true;
      e.target.parentNode.classList.add("is-loop");
      e.target.src = "https://snowleo208.github.io/100-Days-of-Code/7.%20Music%20Player/img/loopone.svg";
      audio.loop = true;
      audio.onended = e => goToNextMusic();
    } else {
      isLoop = false;
      loopOne = false;
      e.target.parentNode.classList.remove("is-loop");
      e.target.src = "https://snowleo208.github.io/100-Days-of-Code/7.%20Music%20Player/img/loop.svg";
      audio.loop = false;
      audio.onended = e => stopMusic();
    }
  };

  const progress = (e) => {
    const audio = document.getElementById(currentAudio);
    const pos = (e.pageX - progressDiv.getBoundingClientRect().x) / progressDiv.getBoundingClientRect().width;
    audio.currentTime = pos * audio.duration;
    changeBar();
  };

  const init = () => {
    const audio = document.getElementById(currentAudio) === null ? new Audio() : document.getElementById(currentAudio);
    audio.src = songList[currentId].src;
    audio.id = currentAudio;
    if (document.getElementById(currentAudio) === null) {
      document.body.appendChild(audio);
    }

    progressBar.style.transition = "none";
    progressBar.style.width = "0%";
    document.getElementById(currentAudio).currentTime = 0;

    title.innerHTML = songList[currentId].title;

    audio.addEventListener("loadedmetadata", function () {
      const leftMinute = Math.floor(audio.duration / 60);
      const leftSecond = Math.floor(audio.duration % 60);
      currentTimeIndicator.innerHTML = "00:00";
      leftTimeIndicator.innerHTML = ("0" + leftMinute).substr(-2) + ":" + ("0" + leftSecond).substr(-2);
      progressBar.style.transition = "";
    });

    document.getElementById(currentAudio).onended = e => goToNextMusic(e);
  };

  playBtn.addEventListener("click", play);
  loopBtn.addEventListener("click", loop);
  shuffleBtn.addEventListener("click", shuffle);
  prevBtn.addEventListener("click", e => nextMusic("prev"));
  nextBtn.addEventListener("click", e => nextMusic("next"));
  progressDiv.addEventListener("click", e => {
    progress(e);
  });

  init();
}
