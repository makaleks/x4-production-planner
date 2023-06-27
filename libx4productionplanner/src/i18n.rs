pub enum UiStringsError {
    LoadingError,
}

fluent_templates::static_loader! {
    static BUILTIN_LOCALES = {
        // Directory of compile-time locales
        locales: "./config/lang",
        // Language to use when some other not filled
        fallback_language: "ru",
        //// Optional: A shared fluent resource
        //core_locales: "./tests/locales/core.ftl",
        //// Optional: A function that is run over each fluent bundle.
        //customise: |bundle| {},
    };
}

pub struct UiStrings {
    lang:             fluent_templates::LanguageIdentifier,
    usr_resource_opt: Option<fluent_templates::ArcLoader>,
}
impl UiStrings {
    pub fn new (lang: fluent_templates::LanguageIdentifier) -> Self {
        Self {
            usr_resource_opt: None,
            lang,
        }
    }
    pub fn set_locale (&mut self, lang: fluent_templates::LanguageIdentifier) {
        self.lang = lang;
    }
    pub fn load (&mut self, path: &std::path::Path) -> Result<(), UiStringsError> {
        let usr_resource
            = fluent_templates::ArcLoader::builder(&path, Default::default())
              .build().map_err(|_| UiStringsError::LoadingError)?;
        self.usr_resource_opt.replace(usr_resource);
        Ok(())
    }
    pub fn get_locales (&self) -> Vec<(String, fluent_templates::LanguageIdentifier)> {
        use fluent_templates::Loader;

        let mut set = std::collections::HashSet::new();
        let mut collection = Vec::new();
        for id in BUILTIN_LOCALES.locales() {
            collection.push((id.to_string(), id.clone()));
            set.insert(id);
        }
        if let Some(usr_resource) = &self.usr_resource_opt {
            for id in usr_resource.locales().filter(|id| !set.contains(id)) {
                collection.push((id.to_string(), id.clone()));
            }
        }
        collection
    }
    pub fn get_string (&self, text_id: &str) -> String {
        use fluent_templates::Loader;

        if let Some(usr_resource) = &self.usr_resource_opt {
            if let Some(string) = usr_resource.lookup_single_language::<&'static str>(&self.lang, text_id, None) {
                return string;
            }
        }
        BUILTIN_LOCALES.lookup_complete::<&'static str>(&self.lang, text_id, None).unwrap_or(format!("<ERROR unset string for key {}>", text_id))
    }
    pub fn get_string_with_args (&self, text_id: &str, args: &std::collections::HashMap<&'static str, fluent_templates::fluent_bundle::FluentValue<'_>>) -> String {
        use fluent_templates::Loader;

        if let Some(usr_resource) = &self.usr_resource_opt {
            if let Some(string) = usr_resource.lookup_single_language(&self.lang, text_id, Some(args)) {
                return string;
            }
        }
        BUILTIN_LOCALES.lookup_complete(&self.lang, text_id, Some(args)).unwrap_or(format!("<ERROR unset string for key {}>", text_id))
    }
}
