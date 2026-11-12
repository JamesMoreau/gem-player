use fully_pub::fully_pub;
use rust_i18n::set_locale;
use serde::{Deserialize, Serialize};
use sys_locale::get_locale;

#[fully_pub]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum LanguagePreference { // TODO: do we need System? could this just be a String.
    System,
    Locale(String),
}

#[fully_pub]
struct LanguageSettings {
    languages: Vec<String>,
    selected: LanguagePreference,
}

impl LanguageSettings {
    pub fn new(selected: LanguagePreference) -> Self {
        Self {
            languages: available_locales!().into_iter().map(|locale| locale.into_owned()).collect(),
            selected,
        }
    }

    pub fn label(&self, preference: &LanguagePreference) -> String {
        match preference {
            LanguagePreference::System => t!("system").to_string(),
            LanguagePreference::Locale(locale) => locale.split('-').next().unwrap_or(locale).to_uppercase(),
        }
    }

    pub fn locale(&self) -> &str {
        match &self.selected {
            LanguagePreference::System => {
                let system_locale = get_locale();

                self.languages
                    .iter()
                    .find(|locale| system_locale.as_ref().is_some_and(|system| locale.eq_ignore_ascii_case(system)))
                    .or_else(|| {
                        let system_language = system_locale.as_deref()?.split('-').next()?;

                        self.languages
                            .iter()
                            .find(|locale| locale.split('-').next().unwrap_or("").eq_ignore_ascii_case(system_language))
                    })
                    .or_else(|| self.languages.first())
                    .map_or("en-US", |locale| locale)
            }
            LanguagePreference::Locale(locale) => locale,
        }
    }

    pub fn apply(&self) {
        set_locale(self.locale());
    }
}
