export interface IInteraction {
  onComplete: ((jumpToLabel: string | null) => void | Promise<void>) | null;
  init(): Promise<void>;
  cleanup?(): void | Promise<void>;
}
