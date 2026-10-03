import { en } from './en';

export type MessageKey = keyof typeof en;

/** Looks up a UI string and fills in its `{name}` placeholders. v1 ships English only. */
export function t(key: MessageKey, params: Record<string, string> = {}): string {
  return en[key].replace(/\{(\w+)\}/g, (placeholder, name: string) => params[name] ?? placeholder);
}
