import { MerchComponent } from "../components/merch.js";

export class HomePage {
  scutoonPages;
  constructor(scutoonPages) {
    this.scutoonPages = scutoonPages;
  }

  init(params = {}) {
    console.log('HomePage initialized', params);
    const merchComponent = new MerchComponent(this.scutoonPages);
    merchComponent.init();
  }
} 