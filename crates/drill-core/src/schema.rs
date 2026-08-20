//! Current wire-schema metadata.
//!
//! AutoDrill is pre-release, so only the current schema is supported. Breaking
//! schema changes replace the previous schema instead of retaining compatibility
//! code until persisted public worksheets/users make that necessary.

pub const SCHEMA_VERSION: u16 = 7;

pub(crate) const fn is_supported_schema_version(schema_version: u16) -> bool {
    schema_version == SCHEMA_VERSION
}
