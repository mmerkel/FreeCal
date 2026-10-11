/** A stand-in for `matchMedia('(prefers-color-scheme: dark)')` that a test can flip. */
export class FakeDarkQuery extends EventTarget {
  constructor(public matches: boolean) {
    super();
  }

  flip(matches: boolean) {
    this.matches = matches;
    this.dispatchEvent(new Event('change'));
  }
}

/** The system's light or dark style in component tests: light unless a test flips it. */
export const systemDarkQuery = new FakeDarkQuery(false);
