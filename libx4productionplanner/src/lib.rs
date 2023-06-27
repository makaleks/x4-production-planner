mod config;
//mod i18n;
mod logic;
mod dataloader;

use config::*;
//use i18n::*;
use dataloader::*;

pub use dataloader::{CountsInput, CountsOutput};

#[derive(Debug)]
pub enum Error {
    ConfigError(ConfigError),
    DataError(DataError),
}
impl From<DataError> for Error {
    fn from(value: DataError) -> Self {
        Self::DataError(value)
    }
}
impl From<ConfigError> for Error {
    fn from(value: ConfigError) -> Self {
        Self::ConfigError(value)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WareRequest {
    pub name:            String,
    pub production_kind: CountsInput,
}
impl WareRequest {
    fn into_tuple (self) -> (String, CountsInput) {
        (self.name, self.production_kind)
    }
}

fn my_count_serializer<S: serde::Serializer>(value: &CountsOutput, serializer: S) -> Result<S::Ok, S::Error> {
    use serde::Serialize;
    value.serialize(serializer)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WareResponse {
    pub name:             String,
    pub wares_per_minute: f64,
    #[serde(serialize_with = "my_count_serializer")]
    pub counts_output:    CountsOutput,
}
pub struct WareResponseExt {
    pub response: WareResponse,
    pub ext:      SingleWare,
}
impl WareResponseExt {
    fn from_tuple ((name, wares_per_minute, counts_output, ext): (String, f64, CountsOutput, SingleWare)) -> Self {
        Self {
            response: WareResponse {name, wares_per_minute, counts_output},
            ext,
        }
    }
}

pub struct X4ProductionPlanner {
    #[allow(dead_code)]
    config: Config,
    //pub strings: UiStrings,
    pub data: Data,
}
impl X4ProductionPlanner {
    //fn new (config: Config) -> Result<(), ()> {
    //}
    pub fn new (gamedir: &std::path::Path) -> Result<Self, Error> {
        Ok(Self {
            config: Config::new(),
            data: Data::load_data(gamedir)?,
        })
    }
    pub fn new_from_data_str (wares_xml_str: String, translation_xml_str: String) -> Result<Self, Error> {
        Ok(Self {
            config: Config::new(),
            data: Data::load_data_str(wares_xml_str, translation_xml_str)?,
        })
    }
    pub fn calc_required_fabric_counts (&mut self, desired_unicode_id_opt: Option<String>, desired_outputs: Vec<WareRequest>, prioritylist: Vec<String>, blacklist: Vec<String>) -> Result<Vec<WareResponseExt>, Error> {
        if let Some(desired_unicode_id) = desired_unicode_id_opt {
            self.data.set_desired_unicode_id(desired_unicode_id);
        }
        let input = desired_outputs.into_iter().map(|v| v.into_tuple()).collect();
        let result = self.data.calc_required_fabric_counts(input, prioritylist, blacklist)?;
        Ok(result.into_iter().map(|v| WareResponseExt::from_tuple(v)).collect())
    }
}

