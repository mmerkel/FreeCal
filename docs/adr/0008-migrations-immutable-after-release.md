# Schema migrations are fixed once a release is tagged

A Local Store records how many schema migrations it has run in `PRAGMA user_version`, and FreeCal runs the rest in order when it opens it. Once a release is tagged, the migrations it shipped are never edited, reordered or removed; schema changes only append new ones. FreeCal can't see which versions are on users' machines, so an edited migration would leave existing Local Stores with a schema the code no longer describes, and no error would say so. Before the first release, the only Local Stores are development ones that can be deleted, so the migrations may be changed freely and are squashed into one baseline just before that release.

## Considered Options

- **Editing migrations whenever it is convenient**: fine while every Local Store is disposable, silently wrong for users after a release.
- **Never squashing, even before the first release**: every release would ship development history, such as the move to wall times (ADR 0007), as code that has to keep working.

## Consequences

- To squash migrations after a release, replace migrations 1 to N with one baseline that sets `user_version` to N directly, and refuse a Local Store between 1 and N−1 with an error that names the FreeCal release that can upgrade it. Local Stores at N or later upgrade as before.
- A Local Store with a higher `user_version` than the app knows is already refused (`NewerLocalStore`), so a development Local Store left from before a squash fails loudly and doesn't get corrupted.
