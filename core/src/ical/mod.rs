//! iCalendar text: the reader and the writer of one CalDAV resource
//! (ADR 0017).

pub(crate) mod read;
#[cfg(test)]
pub(crate) mod server_texts;
mod windows_zones;
pub(crate) mod write;
