// The 3D view's handheld hook (Viewport.svelte registers it): a pose from the phone or a take drives the view's camera,
// release() gives it back to the orbit. → handheldView.api = { pose(sample), release(), camera() } | null; the view says
// when a pose takes its camera and when it has it back → handheldView.onheld(held) (handheld.svelte.js: hh.held).
export const handheldView = { api: null, onheld: null };
