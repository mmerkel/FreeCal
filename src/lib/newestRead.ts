/**
 * Reads of one kind, which can overtake each other: only the newest one
 * started lands (Newest Read, new term).
 */
export class NewestRead {
  #latest = 0;

  /**
   * Reads with `read` and lands its value with `land`, unless a newer read
   * started meanwhile. Rejects only when the newest read fails, so that an
   * older read's failure never overwrites a newer read either.
   */
  async run<T>(read: () => Promise<T>, land: (value: T) => void): Promise<void> {
    const started = ++this.#latest;
    try {
      const value = await read();
      if (started === this.#latest) land(value);
    } catch (error) {
      if (started === this.#latest) throw error;
    }
  }
}
