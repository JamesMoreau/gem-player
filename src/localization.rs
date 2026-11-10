use fully_pub::fully_pub;
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

#[fully_pub]
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Serialize, Deserialize)]
enum LanguagePreference {
    System,
    Eng,
    Fr,
}

impl LanguagePreference {
    pub fn locale(&self) -> &'static str {
        match self {
            Self::System => match sys_locale::get_locale().as_deref() {
                Some(locale) if locale.starts_with("fr") => "fr-CA",
                _ => "en-US",
            },
            Self::Eng => "en-US",
            Self::Fr => "fr-CA",
        }
    }
}


