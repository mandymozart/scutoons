/** Tiny typed event emitter (AR scene → bridge) */
export class Emitter<Events extends { [K in keyof Events]: (...args: any[]) => void }> {
  private listeners = new Map<keyof Events, Set<(...args: any[]) => void>>();

  on<E extends keyof Events>(event: E, listener: Events[E]): () => void {
    if (!this.listeners.has(event)) this.listeners.set(event, new Set());
    this.listeners.get(event)!.add(listener);
    return () => this.listeners.get(event)?.delete(listener);
  }

  emit<E extends keyof Events>(event: E, ...args: Parameters<Events[E]>): void {
    this.listeners.get(event)?.forEach(listener => {
      try {
        listener(...args);
      } catch (error) {
        console.error(`[ArScene] listener for "${String(event)}" failed:`, error);
      }
    });
  }
}
