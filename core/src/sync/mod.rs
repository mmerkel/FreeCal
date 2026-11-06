//! The Sync Engine: it exchanges a Calendar's changes with its server, by
//! Incoming Syncs and, with ticket 17, Outgoing Syncs, and is the only code
//! that changes the Mirror's server data: it is a Guard, and the Local
//! Store's writes of the Mirror take a [`SyncEngineWritePermission`], which
//! only this module can make. Each Provider's part lives in a module of its
//! own, CalDAV's in [`caldav`] (ADR 0013); this module holds what every
//! Provider and both directions share: the Write Permission. A Calendar's
//! Event Rows with one UID belong to the UID's Corresponding Server Object,
//! and only that Server Object writes them (ADR 0011).

pub(crate) mod caldav;

/// The Sync Engine's Write Permission (ADR 0011), which the writes of the
/// Mirror's server data take. Its field is private to this module, so that
/// nothing else can make one.
pub(crate) struct SyncEngineWritePermission(());

/// The [`SyncEngineWritePermission`] the Sync Engine's writes pass.
const WRITE_PERMISSION: &SyncEngineWritePermission = &SyncEngineWritePermission(());
