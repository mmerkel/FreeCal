import type { Settings } from '../core/types';

/**
 * The frontend's own defaults for every setting, the same as a new Local
 * Store's. They apply until the core has answered, and for good when it
 * can't start, so that the startup error screen is drawn with them.
 */
export const DEFAULT_SETTINGS: Settings = { appearance: 'system' };
