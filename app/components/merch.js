import { API_BASE } from '../config.js';

export class MerchComponent {
	scutoonPages;
	constructor(scutoonPages) {
		this.scutoonPages = scutoonPages;
	}

	init() {
		this.initGalleries();
	}

	initGalleries() {
		fetch(`${API_BASE}/merch.json`)
			.then((response) => response.json())
			.then((data) => {
				const galleries = [
					{
						key: "hoodie_black",
						class: ".gallery-hoodie-black",
						label: "Hoodie Black",
					},
					{
						key: "hoodie_beige",
						class: ".gallery-hoodie-beige",
						label: "Hoodie Beige",
					},
					{
						key: "hoodie_grey",
						class: ".gallery-hoodie-grey",
						label: "Hoodie Grey",
					},
					{
						key: "tshirt_white",
						class: ".gallery-tshirt-white",
						label: "T-Shirt White",
					},
					{
						key: "tshirt_black",
						class: ".gallery-tshirt-black",
						label: "T-Shirt Black",
					},
				];

				const offset = 200;
				let delay = 5000;
				galleries.forEach((gallery) => {
					const productData = data[gallery.key];
					const swiperContainer = document.querySelector(gallery.class);
					const wrapper = swiperContainer.querySelector(".swiper-wrapper");

					productData.images.forEach((imagePath) => {
						const slide = document.createElement("div");
						slide.className = "swiper-slide";
						slide.innerHTML = `<img src="${imagePath}" alt="${gallery.label}" />`;
						wrapper.appendChild(slide);
					});

					new Swiper(gallery.class, {
						slidesPerView: 1,
						spaceBetween: 16,
						loop: true,
						autoplay: {
							delay: delay,
							disableOnInteraction: true,
						},
						navigation: {
							nextEl: `${gallery.class} .swiper-button-next`,
							prevEl: `${gallery.class} .swiper-button-prev`,
						},
						breakpoints: {
							768: {
								slidesPerView: 3,
							},
						},
					});
					delay += offset;
				});
			});

	}
}
