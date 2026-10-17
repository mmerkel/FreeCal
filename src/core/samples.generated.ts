// Generated from the Rust core by core/src/typescript.rs. Don't edit;
// run `FREECAL_WRITE_TYPES=1 cargo test -p freecal-core` instead.

import type * as T from './types.generated';
import type { Unbranded } from './types';

/** One serialized sample of each interface type and variant. */
export const samples = {
  AccountId: [
    1
  ],
  Provider: [
    "local"
  ],
  Account: [
    {
      "id": 1,
      "provider": "local"
    }
  ],
  CalendarId: [
    2
  ],
  Colour: [
    "#3a86ff"
  ],
  Calendar: [
    {
      "id": 2,
      "accountId": 1,
      "name": "Home",
      "colour": "#3a86ff",
      "shown": true,
      "readOnly": false
    }
  ],
  EventId: [
    3
  ],
  When: [
    {
      "kind": "allDay",
      "start": "2026-10-05",
      "end": "2026-10-06"
    },
    {
      "kind": "timed",
      "start": "2026-10-05T09:30:00",
      "end": "2026-10-05T10:00:00"
    }
  ],
  EventDraft: [
    {
      "calendarId": 2,
      "title": "Dentist",
      "when": {
        "kind": "timed",
        "start": "2026-10-05T09:30:00",
        "end": "2026-10-05T10:00:00"
      },
      "location": "Main Street 1",
      "description": "Agenda: notes (https://example.org/notes)"
    }
  ],
  Event: [
    {
      "id": 3,
      "calendarId": 2,
      "title": "Dentist",
      "when": {
        "kind": "timed",
        "start": "2026-10-05T09:30:00",
        "end": "2026-10-05T10:00:00"
      },
      "location": {
        "paragraphs": [
          [
            {
              "kind": "text",
              "text": "Main Street 1"
            }
          ]
        ],
        "text": "Main Street 1"
      },
      "description": {
        "paragraphs": [
          [
            {
              "kind": "text",
              "text": "Agenda: "
            },
            {
              "kind": "link",
              "text": "notes",
              "url": "https://example.org/notes"
            }
          ]
        ],
        "text": "Agenda: notes (https://example.org/notes)"
      },
      "pinnedZone": "Europe/Berlin"
    }
  ],
  Occurrence: [
    {
      "eventId": 3,
      "calendarId": 2,
      "title": "Holiday",
      "when": {
        "kind": "allDay",
        "start": "2026-10-05",
        "end": "2026-10-06"
      }
    }
  ],
  StructuredText: [
    {
      "paragraphs": [
        [
          {
            "kind": "text",
            "text": "Agenda: "
          },
          {
            "kind": "link",
            "text": "notes",
            "url": "https://example.org/notes"
          }
        ]
      ],
      "text": "Agenda: notes (https://example.org/notes)"
    }
  ],
  Piece: [
    {
      "kind": "text",
      "text": "Agenda: "
    },
    {
      "kind": "link",
      "text": "notes",
      "url": "https://example.org/notes"
    }
  ],
  Signal: [
    {
      "kind": "calendarsChanged"
    },
    {
      "kind": "occurrencesChanged",
      "calendarIds": [
        2
      ]
    },
    {
      "kind": "dateChanged",
      "date": "2026-10-04"
    }
  ],
  View: [
    "day",
    "week",
    "month"
  ],
  SidebarState: [
    {
      "shown": true,
      "width": null
    },
    {
      "shown": false,
      "width": 18.5
    }
  ],
  Appearance: [
    "system",
    "light",
    "dark"
  ],
  Settings: [
    {
      "appearance": "dark"
    }
  ],
  SettingsUpdate: [
    {},
    {
      "appearance": "light"
    }
  ],
  ImportSummary: [
    {
      "calendarId": 2,
      "newEvents": 40,
      "existingEvents": 2,
      "pinned": [
        {
          "zone": "Europe/Berlin",
          "events": 3
        }
      ]
    }
  ],
  PinnedZone: [
    {
      "zone": "Europe/Berlin",
      "events": 3
    }
  ],
  ExistingEvents: [
    "update",
    "skip",
    "copy"
  ],
  ImportId: [
    4
  ],
  ImportOutcome: [
    {
      "id": 4,
      "imported": 40,
      "updated": 2,
      "skipped": 0
    }
  ],
  ErrorCode: [
    {
      "code": "localStoreFailed"
    },
    {
      "code": "dataDirectoryFailed"
    },
    {
      "code": "localStoreInUse"
    },
    {
      "code": "accountNotFound"
    },
    {
      "code": "calendarNotFound"
    },
    {
      "code": "calendarReadOnly"
    },
    {
      "code": "eventNotFound"
    },
    {
      "code": "endBeforeStart"
    },
    {
      "code": "unreadableEventTime"
    },
    {
      "code": "emptyCalendarName"
    },
    {
      "code": "invalidColour"
    },
    {
      "code": "localAccountCannotBeRemoved"
    },
    {
      "code": "newerLocalStore"
    },
    {
      "code": "unknownProvider"
    },
    {
      "code": "importUnreadable",
      "file": "takeout.zip/Calendar/Work.ics"
    },
    {
      "code": "importNotCalendar",
      "file": "takeout.zip/Calendar/Work.ics"
    },
    {
      "code": "importZipWithoutCalendars",
      "file": "takeout.zip/Calendar/Work.ics"
    },
    {
      "code": "importTooLarge",
      "file": "takeout.zip/Calendar/Work.ics"
    },
    {
      "code": "importNotFound"
    },
    {
      "code": "noFileToImport"
    },
    {
      "code": "linkNotAllowed"
    },
    {
      "code": "linkNotOpened"
    },
    {
      "code": "windowNotShown"
    }
  ],
} satisfies Samples;

type Samples = {
  AccountId: Unbranded<T.AccountId>[];
  Provider: Unbranded<T.Provider>[];
  Account: Unbranded<T.Account>[];
  CalendarId: Unbranded<T.CalendarId>[];
  Colour: Unbranded<T.Colour>[];
  Calendar: Unbranded<T.Calendar>[];
  EventId: Unbranded<T.EventId>[];
  When: Unbranded<T.When>[];
  EventDraft: Unbranded<T.EventDraft>[];
  Event: Unbranded<T.Event>[];
  Occurrence: Unbranded<T.Occurrence>[];
  StructuredText: Unbranded<T.StructuredText>[];
  Piece: Unbranded<T.Piece>[];
  Signal: Unbranded<T.Signal>[];
  View: Unbranded<T.View>[];
  SidebarState: Unbranded<T.SidebarState>[];
  Appearance: Unbranded<T.Appearance>[];
  Settings: Unbranded<T.Settings>[];
  SettingsUpdate: Unbranded<T.SettingsUpdate>[];
  ImportSummary: Unbranded<T.ImportSummary>[];
  PinnedZone: Unbranded<T.PinnedZone>[];
  ExistingEvents: Unbranded<T.ExistingEvents>[];
  ImportId: Unbranded<T.ImportId>[];
  ImportOutcome: Unbranded<T.ImportOutcome>[];
  ErrorCode: Unbranded<T.ErrorCode>[];
};
