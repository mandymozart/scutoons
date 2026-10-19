// The 3D view's handheld hook (Viewport.svelte registers it): a pose from the phone or a take drives the view's camera,
// release() gives it back to the orbit. → handheldView.api = { pose(sample), release(), camera() } | null
export const handheldView = { api: null };
