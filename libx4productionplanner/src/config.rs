use std::str::FromStr;
//use std::collections::HashMap;
use crate::dataloader::check_gamedir;

const BADLANG_STR: &str = "the language string, that follows Unicode Language Identifier (UTS #35: Unicode LDML 3.1 Unicode Language Identifier; crate unic_langid; \"en_US\", \"ru\", etc.)";
//const BADLANGTOIDUSRFIXES_STR: &str = "The `langidfix` must be an array of {lang, id} string pairs";

#[derive(Debug)]
pub enum ConfigError {
    StdIo(std::io::Error),
    BadLang(&'static str),
    BadGamedir(String),
    UnknownPlatform(String),
    NoExeDir,
    BadConfigNotStr,
    BadConfigNotToml,
    BadConfigData,
}
impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        Self::StdIo(value)
    }
}

const DEFAULT_LANG: &'static str = "ru";

fn get_dir_for_platform_default () -> Result<std::path::PathBuf, ConfigError> {
    if cfg!(linux) {
        return Ok("~/.local/share/Steam/steamapps/common/X4 Foundations/".into());
    }
    Err(ConfigError::UnknownPlatform(std::env::consts::OS.into()))
}

fn find_gamedir () -> Result<std::path::PathBuf, ConfigError> {
    let path = get_dir_for_platform_default()?;
    check_gamedir(&path)?;
    Ok(path)
}

fn current_suffix (decor: &toml_edit::Decor) -> &str {
    decor.suffix().map_or(Some(""), |s| s.as_str()).unwrap_or("")
}

// Serde supports neither preserving comments neigther creating new ones
// See https://github.com/serde-rs/serde/issues/1430
//     https://github.com/toml-rs/toml/issues/376
#[derive(Debug, serde::Deserialize)]
pub struct Config {
    // used with getters&setters to update without modifying comments
    #[serde(skip)]
    serialized: toml_edit::Document,
    #[serde(skip)]
    usr_config_path_opt: Option<std::path::PathBuf>, // none if default
    // https://serde.rs/field-attrs.html
    #[serde( deserialize_with = "my_lang_deser",
             default          = "Config::default_lang" )]
    lang: (bool, String, fluent_templates::LanguageIdentifier),

    gamedir: Option<std::path::PathBuf>,

    //#[serde( deserialize_with = "my_lang_to_id_usr_fixes_deser")]
    //#[serde(default)]
    //patch_lang_to_id: HashMap<String, HashMap<String, String>>,
}
#[allow(dead_code)]
impl Config {
    fn new_from_toml (
           serialized:          toml_edit::Document,
           usr_config_path_opt: Option<std::path::PathBuf>
           ) -> Option<Self>
    {
        let mut config
            = toml_edit::de::from_document::<Self>(serialized.clone()).ok()?;

        config.serialized          = serialized;
        config.usr_config_path_opt = usr_config_path_opt;
        Some(config)
    }
    fn default_lang () -> (bool, String, fluent_templates::LanguageIdentifier) {
        (false, DEFAULT_LANG.to_string(), DEFAULT_LANG.parse().unwrap())
    }
    pub fn new () -> Self {
        let mut default_doc = toml_edit::Document::new();

        let lang_key
            = toml_edit::Key::new(
                  "lang"
                  )
              .with_decor(toml_edit::Decor::new(
                  "# the desired language\n",
                  ""
                  ));
        let lang_value = toml_edit::Item::from_str(
                             &format!("\"{}\"", DEFAULT_LANG)
                             ).unwrap();
        default_doc.insert_formatted(&lang_key, lang_value);

        match find_gamedir() {
            Ok(gamedir) => {
                let gamedir_key
                    = toml_edit::Key::new(
                        "gamedir"
                        )
                      .with_decor(toml_edit::Decor::new(
                          "# the game directory\n",
                          ""
                          ));
                let gamedir_value = toml_edit::Item::from_str(
                                        &format!("\"{}\"", gamedir.to_string_lossy())
                                        ).unwrap();
                default_doc.insert_formatted(&gamedir_key, gamedir_value);
            },
            Err(err) => {
                let current = current_suffix(default_doc.decor()).to_string();
                default_doc.decor_mut().set_suffix(format!("{}# gamedir = {:?}\n", current, err));
            },
        }

//        let current = current_suffix(default_doc.decor()).to_string();
//        default_doc.decor_mut().set_suffix(format!("{}#[[langidfix]]\n#lang = \"en\"\n#id = 44\n", current));
        let current = current_suffix(default_doc.decor()).to_string();
        default_doc.decor_mut().set_suffix(format!("{}#[patch_lang_to_id.en]\n#claytronics = \"Claytronics\"\n", current));

        Self::new_from_toml(default_doc, None).unwrap()
    }
    pub fn serialize (&self) -> String {
        self.serialized.to_string()
    }
    pub fn get_config_path (&self) -> &Option<std::path::PathBuf> {
        &self.usr_config_path_opt
    }
    pub fn get_config_path_or_default (&self) -> Result<std::path::PathBuf, ConfigError> {
        match self.usr_config_path_opt.clone() {
            Some (path) => Ok(path),
            None => {
                let mut exe_path = std::env::current_exe()?;
                match exe_path.pop() {
                    true  => Ok(exe_path),
                    false => Err(ConfigError::NoExeDir),
                }
            },
        }
    }
    pub fn load_str (&mut self, input: &str, usr_config_path: Option<std::path::PathBuf>) -> Result<(), ConfigError> {
        let serialized = toml_edit::Document::from_str(&input)
                             .map_err(|_| ConfigError::BadConfigNotToml)?;

        let mut candidate = Self::new_from_toml(
                                serialized, usr_config_path
                                )
                            .ok_or(ConfigError::BadConfigData)?;
        std::mem::swap(self, &mut candidate);
        Ok(())
    }
    pub fn load (&mut self, usr_config_path: std::path::PathBuf) -> Result<(), ConfigError> {
        let file_str_content = std::fs::read_to_string(&usr_config_path)
                                   .map_err(|_| ConfigError::BadConfigNotStr)?;
        self.load_str(&file_str_content, Some(usr_config_path))
    }
    pub fn save (&mut self, usr_config_path: std::path::PathBuf) -> Result<(), ConfigError> {
        std::fs::write(&usr_config_path, self.serialized.to_string())?;
        self.usr_config_path_opt = Some(usr_config_path);
        Ok(())
    }
    pub fn lang (&self) -> &str {
        &self.lang.1
    }
    pub fn lang_code (&self) -> &fluent_templates::LanguageIdentifier {
        &self.lang.2
    }
    pub fn set_lang (&mut self, lang: &str) -> Result<(), ConfigError> {
        let lang_code = lang.parse().map_err(
                            |_| ConfigError::BadLang(BADLANG_STR)
                            )?;
        self.lang = (true, lang.to_string(), lang_code);
        self.serialized.insert(
            "lang",
            toml_edit::Item::Value(
                toml_edit::Value::String(
                    toml_edit::Formatted::new(lang.to_string())
                    ))
            );
        Ok(())
    }
    pub fn gamedir (&self) -> Option<&std::path::Path> {
        self.gamedir.as_deref()
    }
    pub fn set_gamedir (&mut self, gamedir: std::path::PathBuf) -> Result<(), ConfigError> {
        check_gamedir(&gamedir)?;
        self.gamedir = Some(gamedir.clone());
        self.serialized.insert(
            "gamedir",
            toml_edit::Item::Value(
                toml_edit::Value::String(
                    toml_edit::Formatted::new(gamedir.to_string_lossy().into())
                    ))
            );
        Ok(())
    }
}
fn my_lang_deser<'de, D: serde::de::Deserializer<'de>> (de: D) -> Result<(bool, String, fluent_templates::LanguageIdentifier), D::Error> {
    // https://users.rust-lang.org/t/need-help-with-serde-deserialize-with/18374
    struct MyLangVisitor;
    impl<'de> serde::de::Visitor<'de> for MyLangVisitor {
        type Value = (String, fluent_templates::LanguageIdentifier);
        fn expecting (&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str(BADLANG_STR)
        }
        fn visit_str<E: serde::de::Error> (self, v: &str) -> Result<Self::Value, E> {
            let id = v.parse().map_err(|_| (E::invalid_value(serde::de::Unexpected::Str(v), &self)))?;
            Ok((v.to_string(), id))
        }
    }
    let (serialized, id) = de.deserialize_str(MyLangVisitor)?;
    Ok((true, serialized, id))
}
//fn my_lang_to_id_usr_fixes_deser<'de, D: serde::de::Deserializer<'de>> (de: D) -> Result<HashMap<String, String>, D::Error> {
//    // https://users.rust-lang.org/t/need-help-with-serde-deserialize-with/18374
//    struct MyLangToIdUsrFixesMapVisitor;
//    impl<'de> serde::de::Visitor<'de> for MyLangToIdUsrFixesMapVisitor {
//        type Value = (String, String);
//        fn expecting (&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//            formatter.write_str(BADLANGTOIDUSRFIXES_STR)
//        }
//        fn visit_map<A>(mut self, map: A) -> Result<Self::Value, A::Error>
//            where
//                A: serde::de::MapAccess<'de>,
//        {
//            use serde::de::Error;
//            let id_key = "id";
//            let tr_key = "tr";
//            let (key1, value1): (&str, &str) = map.next_entry()?.ok_or(A::Error::missing_field("id&tr"))?;
//            if key1 != id_key && key1 != tr_key {
//                return Err(A::Error::invalid_value(serde::de::Unexpected::Map, &"expected 'id' or 'tr'"));
//            }
//            let (key2, value2): (&str, &str) = map.next_entry()?.ok_or(A::Error::missing_field("id&tr"))?;
//            if key2 != id_key && key1 != tr_key {
//                return Err(A::Error::invalid_value(serde::de::Unexpected::Map, &"expected 'id' or 'tr'"));
//            }
//            else if key2 == key1 {
//                return Err(A::Error::duplicate_field("key1"));
//            }
//            if let Ok(Some((key, value))) = map.next_entry::<&str, &str>() {
//                return Err(A::Error::unknown_field(key, &[id_key, tr_key]));
//            }
//            let (key, value) = if key1 == id_key {
//                                   (value1, value2)
//                               }
//                               else {
//                                   (value2, value1)
//                               };
//            Ok((key.to_string(), value.to_string()))
//        }
//    }
//    let deserialized = de.deserialize_map(MyLangToIdUsrFixesMapVisitor)?;
//    Ok(deserialized)
//}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo() {
        let s = "";
        println!("raw str: \"{:?}\"", toml_edit::de::from_str::<Config>(s));

        let mut doc = toml_edit::Document::new();
        let lang_key = toml_edit::Key::new("lang").with_decor(toml_edit::Decor::new("# the desired language\n", ""));
        let lang_value = std::str::FromStr::from_str("\"ru\"").unwrap();
        doc.insert_formatted(&lang_key, lang_value);
        println!("Serialized: \"\n{}\n\"", doc.to_string());
        println!("Parsed: \"{:?}\"", toml_edit::de::from_document::<Config>(doc));
    }
    #[test]
    fn test_config () {
        let default_config = Config::new();
        println!("# default config: {:?}\n  Serialized: \"{}\"", default_config, default_config.serialize());

        let demo_config_1_input = r#"
# my comment
lang = "ru"
        "#;
        let demo_config_1
            = { let mut config = Config::new();
                config.load_str(demo_config_1_input, None).unwrap();
                config };
        println!("# 1 config {:?}\n    serialzied {}", demo_config_1, demo_config_1.serialize());
    }
}
