import type { ErrorCode } from '../core/types';
import { en } from './en';

export type MessageKey = keyof typeof en;

/** Looks up a UI string and fills in its `{name}` placeholders. v1 ships English only. */
export function t(key: MessageKey, params: Record<string, string> = {}): string {
  return en[key].replace(/\{(\w+)\}/g, (placeholder, name: string) => params[name] ?? placeholder);
}

/** The words for each code a failure crosses the core interface as. */
const errorMessages: Record<ErrorCode['code'], MessageKey> = {
  localStoreFailed: 'error.localStoreFailed',
  dataDirectoryFailed: 'error.dataDirectoryFailed',
  localStoreInUse: 'error.localStoreInUse',
  accountNotFound: 'error.accountNotFound',
  calendarNotFound: 'error.calendarNotFound',
  calendarReadOnly: 'error.calendarReadOnly',
  eventNotFound: 'error.eventNotFound',
  endBeforeStart: 'error.endBeforeStart',
  unreadableEventTime: 'error.unreadableEventTime',
  emptyCalendarName: 'error.emptyCalendarName',
  invalidColour: 'error.invalidColour',
  localAccountCannotBeRemoved: 'error.localAccountCannotBeRemoved',
  newerLocalStore: 'error.newerLocalStore',
  unknownProvider: 'error.unknownProvider',
  linkNotAllowed: 'error.linkNotAllowed',
  linkNotOpened: 'error.linkNotOpened',
  windowNotShown: 'error.windowNotShown',
};

/** The startup error screen's own words for why the core couldn't open. */
const startupMessages: Partial<Record<ErrorCode['code'], MessageKey>> = {
  localStoreInUse: 'startup.localStoreInUse',
  newerLocalStore: 'startup.newerLocalStore',
  localStoreFailed: 'startup.localStoreFailed',
  dataDirectoryFailed: 'startup.dataDirectoryFailed',
};

/** Why FreeCal couldn't start, in words, for the startup error screen. */
export function startupText(failure: ErrorCode): string {
  const key = startupMessages[failure.code];
  return key ? t(key) : t('app.loadFailed', { error: errorText(failure) });
}

/**
 * A failure in words, for the `{error}` of a message. The core reports
 * failures as codes; anything else, such as a failure inside the frontend, is
 * shown as it is.
 */
export function errorText(error: unknown): string {
  if (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    typeof error.code === 'string' &&
    Object.hasOwn(errorMessages, error.code)
  ) {
    return t(errorMessages[error.code as ErrorCode['code']]);
  }
  return String(error);
}
