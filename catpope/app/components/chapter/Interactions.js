export const initInteractions = (chapterData) => {
  console.log("Initializing interactions for chapter:", chapterData.id);
  // TODO load interactions dynamically based on chapterData
  // An interaction blocks the continous flow of slices rendered for a chapter.
  // for an interaction segment/group, the interaction defines how to pick and render slices
  // is no interaction defined within a group or is no group defined, slices are rendered continously
  // and no interaction is applied.
  // the interaction can be defined at chapter level or at slice group level, for basically 
  // a chapter is the same as a group in terms of how interactions are applied, 
  // no interaction means, the default continous rendering of slices. TODO this has to be adopted to fit the 
  // new dogma. I will have basic scroll (current), advancedScroll (horizontal and vertical, eventually infinite),
  // no scroll (background slices as static image/video like a graphic novel), and possibly other interactions.
};
