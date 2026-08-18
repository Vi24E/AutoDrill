//! Current wire-schema metadata.
//!
//! AutoDrill is pre-release, so only the current schema is supported. Breaking
//! schema changes replace the previous schema instead of retaining compatibility
//! code until persisted public worksheets/users make that necessary.

pub const SCHEMA_VERSION: u16 = 7;
pub const OPERATION_KIND_COUNT: usize = 32;
pub const SUPPORTED_SCHEMA_VERSIONS: [u16; 1] = [SCHEMA_VERSION];

pub const fn is_supported_schema_version(schema_version: u16) -> bool {
    schema_version == SCHEMA_VERSION
}

pub const fn operation_kind_count_for_schema(schema_version: u16) -> Option<usize> {
    if schema_version == SCHEMA_VERSION {
        Some(OPERATION_KIND_COUNT)
    } else {
        None
    }
}
