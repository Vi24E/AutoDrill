use crate::effort::{OperationWeights, WeightProfile};
use crate::generator::registered_generator_entries;
use crate::theme::ThemeRegistration;

/// Resolve the current registration owned by the theme/family module.
pub fn active_registration(numeric_theme_id: u32) -> Option<&'static ThemeRegistration> {
    registered_generator_entries()
        .map(|entry| entry.generator)
        .find(|generator| generator.registration().numeric_theme_id == numeric_theme_id)
        .map(|generator| generator.registration())
}

pub fn registration(
    numeric_theme_id: u32,
    generator_revision: u32,
) -> Option<&'static ThemeRegistration> {
    registered_generator_entries()
        .map(|entry| entry.generator)
        .find(|generator| {
            let registration = generator.registration();
            registration.numeric_theme_id == numeric_theme_id
                && registration.generator_revision == generator_revision
        })
        .map(|generator| generator.registration())
}

/// Current registrations sorted by numeric theme identity.
pub fn active_registrations() -> Vec<&'static ThemeRegistration> {
    use std::collections::BTreeMap;

    let mut registrations = BTreeMap::<u32, &'static ThemeRegistration>::new();
    for entry in registered_generator_entries() {
        let registration = entry.generator.registration();
        let previous = registrations.insert(registration.numeric_theme_id, registration);
        assert!(
            previous.is_none(),
            "a numeric theme ID must have exactly one current generator"
        );
    }
    registrations.into_values().collect()
}

pub fn resolved_weights(registration: &ThemeRegistration) -> OperationWeights {
    let mut profile = WeightProfile::default();
    for &(kind, multiplier) in registration.operation_weight_overrides {
        profile
            .theme
            .override_multiplier(kind, multiplier)
            .expect("registry weight multiplier must be finite and nonnegative");
    }
    profile.resolve(&OperationWeights::default())
}
