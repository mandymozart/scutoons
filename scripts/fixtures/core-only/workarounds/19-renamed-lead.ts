// workaround 19: the adaptive lead under another name
export class OutputCushion {
	frames = 2048;
	underrun() {
		this.frames = Math.min(this.frames + 512, 6144);
	}
}
