export class StrollInteraction {
  // params
  // after (label to jump to after interaction)
  // startTime
  // endTime (automatically jump to the after label)
  //
  constructor(params) {
    // Implement stroll interaction logic
    // Stroll will pick a random scene from within a slice group,
    // a slice group is an array of scenes within a slice
    //
    this.params = params;
    console.log("StrollInteraction initialized with params:", params);
  }
  // Hard code interactions here
  init(params) {
    console.log("Starting stroll interaction");
  }
}
