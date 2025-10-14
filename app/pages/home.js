import { MerchComponent } from "../components/merch.js";

export class HomePage {
  scutoonPages;
  constructor(scutoonPages) {
    this.scutoonPages = scutoonPages;
  }

  init() {
    console.log('HomePage initialized');
    const merchComponent = new MerchComponent(this.scutoonPages);
    merchComponent.init();
  }
} 