use std::collections::BTreeMap;
use std::sync::LazyLock;

use thiserror::Error;

use crate::generator::{registered_generator_entries, GeneratorEntry, ProblemGenerator};
use crate::theme::ThemeRegistration;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum RegistryError {
    #[error("numeric theme ID {numeric_theme_id} is registered more than once")]
    DuplicateThemeId { numeric_theme_id: u32 },
}

struct Registry {
    by_theme: BTreeMap<u32, GeneratorEntry>,
}

impl Registry {
    fn build(entries: impl IntoIterator<Item = GeneratorEntry>) -> Result<Self, RegistryError> {
        let mut by_theme = BTreeMap::new();
        for entry in entries {
            let numeric_theme_id = entry.generator.registration().numeric_theme_id();
            if by_theme.insert(numeric_theme_id, entry).is_some() {
                return Err(RegistryError::DuplicateThemeId { numeric_theme_id });
            }
        }
        Ok(Self { by_theme })
    }

    fn current(&self, numeric_theme_id: u32) -> Option<GeneratorEntry> {
        self.by_theme.get(&numeric_theme_id).copied()
    }
}

// Static registration is programmer-authored configuration. Keep its validation
// result rather than panicking on first access; every public boundary propagates
// a RegistryError if the configuration is inconsistent.
static REGISTRY: LazyLock<Result<Registry, RegistryError>> =
    LazyLock::new(|| Registry::build(registered_generator_entries()));

fn registry() -> Result<&'static Registry, RegistryError> {
    REGISTRY.as_ref().map_err(|error| *error)
}

/// Resolve the current registration owned by the theme/family module.
pub fn active_registration(
    numeric_theme_id: u32,
) -> Result<Option<&'static ThemeRegistration>, RegistryError> {
    Ok(registry()?
        .current(numeric_theme_id)
        .map(|entry| entry.generator.registration()))
}

pub fn registration(
    numeric_theme_id: u32,
    generator_revision: u32,
) -> Result<Option<&'static ThemeRegistration>, RegistryError> {
    Ok(active_registration(numeric_theme_id)?
        .filter(|registration| registration.generator_revision() == generator_revision))
}

pub(crate) fn generator_for_revision(
    numeric_theme_id: u32,
    generator_revision: u32,
) -> Result<Option<&'static dyn ProblemGenerator>, RegistryError> {
    Ok(registry()?
        .current(numeric_theme_id)
        .map(|entry| entry.generator)
        .filter(|generator| generator.registration().generator_revision() == generator_revision))
}

/// Current registrations sorted by numeric theme identity.
pub fn active_registrations() -> Result<Vec<&'static ThemeRegistration>, RegistryError> {
    Ok(registry()?
        .by_theme
        .values()
        .map(|entry| entry.generator.registration())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes::basic_arithmetic::ONE_DIGIT_ADDITION_GENERATOR;

    #[test]
    fn registry_builder_rejects_duplicate_current_theme_ids() {
        let duplicate = GeneratorEntry::current(&ONE_DIGIT_ADDITION_GENERATOR);
        assert_eq!(
            Registry::build([duplicate, duplicate]).err(),
            Some(RegistryError::DuplicateThemeId {
                numeric_theme_id: duplicate.generator.registration().numeric_theme_id(),
            })
        );
    }

    #[test]
    fn registry_exposes_only_each_theme_current_revision() {
        for current in active_registrations().unwrap() {
            let theme_id = current.numeric_theme_id();
            let revision = current.generator_revision();
            assert_eq!(
                super::registration(theme_id, revision).unwrap(),
                Some(current)
            );
            assert!(generator_for_revision(theme_id, revision)
                .unwrap()
                .is_some());

            if let Some(previous) = revision.checked_sub(1).filter(|value| *value > 0) {
                assert!(super::registration(theme_id, previous).unwrap().is_none());
                assert!(generator_for_revision(theme_id, previous)
                    .unwrap()
                    .is_none());
            }
            if let Some(next) = revision.checked_add(1) {
                assert!(super::registration(theme_id, next).unwrap().is_none());
                assert!(generator_for_revision(theme_id, next).unwrap().is_none());
            }
        }
    }
}
