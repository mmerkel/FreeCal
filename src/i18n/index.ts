import type { ErrorCode } from '../core/types';
import { en } from './en';

export type MessageKey = keyof typeof en;

/** Looks up a UI string and fills in its `{name}` placeholders. v1 ships English only. */
export function t(key: MessageKey, params: Record<string, string> = {}): string {
  return en[key].replace(/\{(\w+)\}/g, (placeholder, name: string) => params[name] ?? placeholder);
}

/** The keys of messages about a count, without their `.one` and `.other`. */
export type CountKey = WithoutOne<MessageKey>;
type WithoutOne<Key> = Key extends `${infer Base}.one` ? Base : never;

/**
 * A message about `count` things, in its form for one or for any other
 * number, with `{count}` and its other placeholders filled in.
 */
export function tCount(key: CountKey, count: number, params: Record<string, string> = {}): string {
  const form = new Intl.PluralRules('en').select(count) === 'one' ? 'one' : 'other';
  return t(`${key}.${form}`, { ...params, count: String(count) });
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
  invalidRepeat: 'error.invalidRepeat',
  occurrenceNotFound: 'error.occurrenceNotFound',
  unreadableEventTime: 'error.unreadableEventTime',
  emptyCalendarName: 'error.emptyCalendarName',
  invalidColour: 'error.invalidColour',
  localAccountCannotBeRemoved: 'error.localAccountCannotBeRemoved',
  newerLocalStore: 'error.newerLocalStore',
  unknownProvider: 'error.unknownProvider',
  importUnreadable: 'error.importUnreadable',
  importNotCalendar: 'error.importNotCalendar',
  importZipWithoutCalendars: 'error.importZipWithoutCalendars',
  importTooLarge: 'error.importTooLarge',
  unitNotFound: 'error.unitNotFound',
  changedSinceUnit: 'error.changedSinceUnit',
  alreadyUndone: 'error.alreadyUndone',
  noFileToImport: 'error.noFileToImport',
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
 * failures as codes, some with the name of the file they are about; anything
 * else, such as a failure inside the frontend, is shown as it is.
 */
export function errorText(error: unknown): string {
  if (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    typeof error.code === 'string' &&
    Object.hasOwn(errorMessages, error.code)
  ) {
    const params: Record<string, string> =
      'file' in error && typeof error.file === 'string' ? { file: error.file } : {};
    return t(errorMessages[error.code as ErrorCode['code']], params);
  }
  return String(error);
}
