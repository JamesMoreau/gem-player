use fully_pub::fully_pub;
use rust_i18n::set_locale;
use sys_locale::get_locale;

#[fully_pub]
struct LanguageSettings {
    languages: Vec<String>,
    selected_locale: Option<String>,
}

pub fn language_label(locale: Option<&str>) -> String {
    match locale {
        None => t!("system").to_string(),
        Some(locale) => locale.split('-').next().unwrap_or(locale).to_uppercase(),
    }
}

impl LanguageSettings {
    pub fn new(selected_locale: Option<String>) -> Self {
        let languages: Vec<String> = available_locales!().into_iter().map(|locale| locale.into_owned()).collect();

        let selected_locale = selected_locale.filter(|locale| languages.contains(locale));

        Self {
            languages,
            selected_locale,
        }
    }

    pub fn locale(&self) -> &str {
        let Some(selected_locale) = &self.selected_locale else {
            let system_locale = get_locale();

            return self
                .languages
                .iter()
                .find(|locale| system_locale.as_ref().is_some_and(|system| locale.eq_ignore_ascii_case(system)))
                .or_else(|| {
                    let system_language = system_locale.as_deref()?.split('-').next()?;

                    self.languages
                        .iter()
                        .find(|locale| locale.split('-').next().unwrap_or("").eq_ignore_ascii_case(system_language))
                })
                .or_else(|| self.languages.first())
                .map_or("en-US", |locale| locale);
        };

        selected_locale
    }

    pub fn apply(&self) {
        set_locale(self.locale());
    }
}
