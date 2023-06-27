use libx4productionplaner::*;

#[derive(Debug)]
enum InputError {
    NoWaresInput,
    CsvError(csv::Error),
    TomlError(toml_edit::de::Error),
}
impl From<csv::Error> for InputError {
    fn from(value: csv::Error) -> Self {
        Self::CsvError(value)
    }
}
impl From<toml_edit::de::Error> for InputError {
    fn from(value: toml_edit::de::Error) -> Self {
        Self::TomlError(value)
    }
}

//enum DemoError {
//    LibError(Error),
//    InputError(InputError),
//}

pub fn get_substr_byte_offset (sub_str: &str, base_str: &str) -> Option<usize> {
    // I couldn't find a Rust alternative to C obvious method
    unsafe {
        //use std::convert::TryFrom;
        let offset_option = usize::try_from(
            sub_str.as_ptr().offset_from(base_str.as_ptr())
        ).ok();
        match offset_option {
            Some(offset) if offset <= base_str.as_bytes().len() => {
                offset_option
            }
            _ => None
        }
    }
}

fn read_csv (csv: &str) -> Result<Vec<WareRequest>, InputError> {
    // https://docs.rs/csv/latest/csv/cookbook/index.html
    let mut reader_builder = csv::ReaderBuilder::new();
    reader_builder.delimiter(';' as u8);
    let mut reader = reader_builder.from_reader(csv.as_bytes());

    let mut result = Vec::new();
    for line_res in reader.deserialize() {
        let record = line_res?;
        result.push(record);
    }
    Ok(result)
}
fn write_csv<T: serde::Serialize> (wares: &[T]) -> Result<String, std::io::Error> {
    let mut data = Vec::<u8>::new();
    {
        let mut builder = csv::WriterBuilder::new();
        builder.delimiter(';' as u8);
        let mut writer = builder.from_writer(&mut data);
        for w in wares {
            writer.serialize(w)?;
        }
        writer.flush()?;
    }
    Ok(String::from_utf8(data).unwrap())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct InputMeta {
    desired_unicode_id: Option<String>,
    #[serde(default)]
    prioritylist:       Vec<String>,
    #[serde(default)]
    blacklist:          Vec<String>,
}
struct Input {
    meta:         InputMeta,
    ware_request: Vec<WareRequest>,
}
impl Input {
    fn load (input_str: String) -> Result<Self, InputError> {
        for remaining in input_str.lines().map(|line| &input_str[get_substr_byte_offset(line, &input_str).unwrap()..]) {
            if let Ok(ware_request) = read_csv(remaining) {
                let meta = toml_edit::de::from_str(&input_str[..get_substr_byte_offset(remaining, &input_str).unwrap()])?;
                return Ok(Self{meta, ware_request});
            }
        }
        Err(InputError::NoWaresInput)
    }
}

#[derive(Debug, clap::Args)]
struct ArgsRequest {
    #[arg(short, long)]
    gamedir: std::path::PathBuf,
    #[arg(short, long)]
    request_file: std::path::PathBuf,
}

#[derive(Debug, clap::Parser)]
#[command(about = "From file-based request and gamedir prints fabric components for your X4 game")]
enum Args {
    Request(ArgsRequest),
    ExampleRequest,
}
#[derive(Debug)]
enum InnerArgsWithGameKind {
    Request(std::path::PathBuf),
}

#[derive(Debug)]
struct InnerArgsWithGame {
    gamedir: std::path::PathBuf,
    kind: InnerArgsWithGameKind,
}

#[derive(Debug)]
enum InnerArgs {
    ExampleRequest,
    WithGame(InnerArgsWithGame),
}

impl From<Args> for InnerArgs {
    fn from(value: Args) -> Self {
        match value {
            Args::ExampleRequest => Self::ExampleRequest,
            Args::Request(ArgsRequest{gamedir, request_file}) => Self::WithGame(InnerArgsWithGame{gamedir, kind: InnerArgsWithGameKind::Request(request_file)}),
        }
    }
}

fn main () {
    use clap::Parser;
    //println!("hello-1");
    let args = Args::parse();
    //println!("hello-2");
    //println!("{:?}", args);
    let args = InnerArgs::from(args);

    match args {
        InnerArgs::ExampleRequest => {
            let example
                = Input {
                     meta: InputMeta {
                               desired_unicode_id: Some("en".into()),
                               prioritylist: vec!["Universal".into()],
                               blacklist: vec!["Teladi".into()],
                           },
                     ware_request: vec![
                         WareRequest{name: "Microchips".into(), production_kind: CountsInput::WaresPerMinute(100.)},
                         WareRequest{name: "ARG S All-round Engine Mk1".into(), production_kind: CountsInput::Fabrics(1)},
                         ],
                     };
            let csv_str = write_csv(&example.ware_request).unwrap();
            //println!("deser: {:?}", read_csv(csv_str.trim()).unwrap());
            let toml_str = toml_edit::ser::to_string_pretty(&example.meta).unwrap();
            let example_request = format!("# These fields are optional\n{}# This csv is required\n{}", toml_str, csv_str);
            print!("{}", example_request);
        }
        InnerArgs::WithGame(InnerArgsWithGame{gamedir, kind})
            => {
                let mut planner = X4ProductionPlanner::new(&gamedir).unwrap();
                match kind {
                    InnerArgsWithGameKind::Request(request_file_path) => {
                        let content = std::fs::read_to_string(request_file_path).unwrap();
                        let input = Input::load(content).unwrap();

                        let result_ext
                            = planner.calc_required_fabric_counts(
                                  input.meta.desired_unicode_id,
                                  input.ware_request,
                                  input.meta.prioritylist,
                                  input.meta.blacklist
                                  ).unwrap();
                        let result = result_ext.into_iter().map(|r_ext| r_ext.response).collect::<Vec<_>>();
                        let result_str = write_csv(&result).unwrap();
                        println!("{}", result_str);
                    }
                }
            }
    }
}
