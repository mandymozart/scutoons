// The 3D view's handheld hook (Viewport.svelte registers it): a pose from the phone or a take drives the view's camera,
// release() gives it back to the orbit; still(canvas, sample?) draws the view – at that pose, without taking the camera –
// into a 2D canvas (the trim's in / out pair). → handheldView.api = { pose(sample), release(), camera(), still } | null; the view says
// when a pose takes its camera and when it has it back → handheldView.onheld(held) (handheld.svelte.js: hh.held).
export const handheldView = { api: null, onheld: null };
