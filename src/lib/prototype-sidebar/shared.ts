// PROTOTYPE. The bits all variants need: the palette and the props.
import type { CoreApi } from '../../core/CoreApi';
import type { Account, Calendar } from '../../core/types';

export const PALETTE = ['#d50000', '#e67c73', '#f4511e', '#f6bf26', '#33b679', '#0b8043', '#039be5', '#3f51b5', '#7986cb', '#8e24aa', '#616161', '#795548'];

export interface VariantProps {
  core: CoreApi;
  accounts: Account[];
  calendars: Calendar[];
}

export const accountName = (account: Account) => (account.provider === 'local' ? 'Local' : account.provider);
