import { Slice, ChapterRef, LoadedSequenceItem } from '../models/content';

export interface IRenderer {
  render(options: RenderOptions): Promise<void>;
  cleanup(): Promise<void>;
}

export interface RenderOptions {
  slices?: Slice[];
  chapter?: any;
  startIndex?: number;
}
