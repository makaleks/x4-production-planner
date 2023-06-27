use std::cell::Cell;
use std::collections::HashMap;

const TRANSLATIONS_FILE: &str = "09.dat";
const PRODUCTION_FILE:   &str = "08.dat";

/*
// I have no idea how to map this in runtime
// 09.dat -> <page id="20201" title="Wares" ..>
// 08.dat -> <wares>
const WARE_ID_TO_TRANSLATION_ID: [(&str, &str); _] = [
    ("advancedcomposites", "401"),
    ("advancedelectronics", "101"),
    ("antimattercells", "201"),
    ("antimatterconverters", "301"),
    ("claytronics", "501"),
    ("dronecomponents", "601"),
    ("energycells", "701"),
    // ...
];
// Naive
// See 08.dat -> <languages>
const LANGUAGE_ID_TO_LOCALE_ID: [(&str, &str); 12] = [
    ("44", "en"), // english
    ("49", "de"), // german
    ("33", "fr"), // french
    ("39", "it"), // italian
    ( "7", "ru"), // russian
    ("34", "sp"), // spanish
    ("55", "pt"), // portuguese (brazil)
    ("48", "pl"), // polish
    ("86", "ch"), // simplified chinese
    ("88", "ch"), // traditional chinese (sorry, no idea how Unicode marks them)
    ("82", "kr"), // korean
    ("81", "jp"), // japanese
];
*/

#[derive(Debug)]
pub enum DataError {
    StdIo(std::io::Error),
    InvalidXml(std::path::PathBuf, roxmltree::Error),
    AttributeNotFound(&'static str, String),
    AttributeBadValue(&'static str, String, &'static str),
    TagNotFound(&'static str, String),
    DuplicateValue(String, String),
    KeyNotFound(String, String),
    XmlPartNotFound(&'static str),
    NoChildren(String),
    SubstrNotFound(&'static str),
    EmptyResultForItem(&'static str),
    DefaultLangTranslationsNotFound,

    ReverseTranslationNotFound(String),
    PriorityBlackListsIntersection(String),
    UnknownProductionMethod(String),

    UnknownWare(String),
    AllMethodsBlacklisted(String),
    NoProductionMethods(String),

    InconsistentRequest(&'static str),

    TranslationError(String),
}
impl From<std::io::Error> for DataError {
    fn from(value: std::io::Error) -> Self {
        Self::StdIo(value)
    }
}

fn mygetatr<'a> (node: roxmltree::Node<'a, 'static>, atr: &'static str) -> Result<&'a str, DataError> {
    node.attribute(atr).ok_or(
        DataError::AttributeNotFound(atr, format!("{:?}", node))
        )
}
fn mygetatrparsed<'a, F: std::str::FromStr> (node: roxmltree::Node<'a, 'static>, atr: &'static str) -> Result<F, DataError> {
    mygetatr(node, atr)?.parse().map_err(
        |_|
        DataError::AttributeBadValue(
            atr,
            format!("{:?}", node),
            std::any::type_name::<F>()
            )
        )
}
fn myfindchildtag<'a> (node: roxmltree::Node<'a, 'static>, tag: &'static str) -> Result<roxmltree::Node<'a, 'static>, DataError> {
    node.children().find(|n|n.has_tag_name(tag))
        .ok_or(DataError::TagNotFound(tag, format!("{:?}", node)))
}

pub fn check_gamedir (path: &std::path::Path) -> Result<(), crate::config::ConfigError> {
    use crate::config::ConfigError;

    let mut path = path.to_path_buf();
    path.push(TRANSLATIONS_FILE);
    if !path.is_file() {
        return Err(ConfigError::BadGamedir("translation-file".into()));
    }

    let mut path = path.to_path_buf();
    path.push(PRODUCTION_FILE);
    if !path.is_file() {
        return Err(ConfigError::BadGamedir("production-file".into()));
    }

    Ok(())
}

fn load_string_file (dir: &std::path::Path, file: &str) -> Result<(String, std::path::PathBuf), DataError> {
    let mut path = dir.to_path_buf();
    path.push(file);
    let content_bytes = std::fs::read(&path)?;
    let content = String::from_utf8_lossy(&content_bytes).into();
    Ok((content, path))
}
fn read_xml<Finder: Fn(&str)->Option<(&str, &str)>>(input_owned: String, finder_to_iter: Finder, dbg_name: &'static str, dbg_path: &std::path::Path) -> Result<Cell<(String, Vec<roxmltree::Document<'static>>)>, DataError> {
    let mut result_xmls = Vec::new();
    let mut remaining = input_owned.as_str();
    while let Some((new_remaining, content_slice)) = finder_to_iter(&remaining) {
        let document = roxmltree::Document::parse(
                           // Safety: since we put String to Cell, the position
                           //         on heap is persistent, cell marks that
                           //         data and its reference change
                           //         simultaniously
                           unsafe{&*(content_slice as * const _)}
                           ).map_err(|e| DataError::InvalidXml(dbg_path.to_path_buf(), e))?;
        result_xmls.push(document);
        remaining = new_remaining;
    }
    if result_xmls.is_empty() {
        Err(DataError::XmlPartNotFound(dbg_name))
    }
    else {
        Ok(Cell::new((input_owned, result_xmls)))
    }
}

fn find_translation_xml_slice (input: &str) -> Option<(&str, &str)> {
    while !input.is_empty() {
        let key1 = "<language";
        let key1_idx = input.find(key1)?;
        let after_key1 = &input[key1_idx+key1.len()..];
        let key2 = "</language>";
        let end_idx = key1_idx + key1.len() + after_key1.find(key2)? + key2.len();
        return Some((&input[end_idx..].trim_start(), &input[key1_idx..end_idx]));
    }
    None
}
fn find_wares_xml_slice (mut input: &str) -> Option<(&str, &str)> {
    while !input.is_empty() {
        let key1 = "<wares>";
        let key1_idx = input.find(key1)?;
        let after_key1 = &input[key1_idx+key1.len()..];
        if after_key1.trim_start().starts_with("<production>") {
            let key2 = "</wares>";
            let end_idx = key1_idx + key1.len() + after_key1.find(key2)? + key2.len();
            return Some((&input[end_idx..end_idx], &input[key1_idx..end_idx]));
        }
        input = after_key1;
    }
    None
}
//fn en_string_to_ware_name (text: &str) -> String {
//    String::from_iter(
//        text.chars()
//        .filter(|c| !c.is_ascii_whitespace())
//        .map(|c| c.to_ascii_lowercase())
//        )
//}

#[derive(Debug, Clone)]
struct TranslationPos {
    page: String,
    id:   String,
}

#[derive(Debug, Clone)]
pub struct SingleWareProduction {
    pub method:             String,
    pub cicle_seconds:      f64, // found thing which is 1.5 sec
    pub wares_per_cicle:    usize,
    pub wares_dependencies: Vec<(String, usize)>,

    translation: TranslationPos,
}
impl SingleWareProduction {
    pub fn wares_per_minute (&self) -> f64 {
        self.wares_per_cicle as f64 / (self.cicle_seconds as f64 / 60f64)
    }
    pub fn dependencies_per_minute (&self) -> impl Iterator<Item=(&str, f64)> {
        self.wares_dependencies.iter().map(|(name, wares_per_cicle)| (name.as_str(), *wares_per_cicle as f64 / (self.cicle_seconds as f64 / 60f64)))
    }
    pub fn fabrics_count_from_desired_wares_per_minute (&self, desired_wares_per_minute: f64) -> usize {
        (desired_wares_per_minute / self.wares_per_minute()).ceil() as usize
    }
}
#[derive(Debug, Clone)]
pub struct SingleWareInfo {
    pub ware_id:         String,

    // might use in future?
    pub price_min:     u32,
    pub price_max:     u32,
    pub transport:     String, // container vs .?.
}
#[derive(Debug, Clone)]
pub struct SingleWare {
    pub info: SingleWareInfo,

    // methods: 'default', 'teladi'
    pub production_methods: Vec<(String, SingleWareProduction)>,

    translation: Option<TranslationPos>,
}
impl SingleWare {
    fn find_desired_method (&self, prioritylist: &[String], blacklist: &[String]) -> Result<&SingleWareProduction, DataError> {
        let methods = &self.production_methods;

        if methods.is_empty() {
            return Err(DataError::NoProductionMethods(self.info.ware_id.clone()));
        }

        for p in prioritylist {
            if let Some((_, method)) = methods.iter().find(|(key, _)| key == p) {
                return Ok(method);
            }
        }
        for (name, method) in methods {
            if let None = blacklist.iter().find(|&b_item| b_item == name) {
                return Ok(method);
            }
        }
        Err(DataError::AllMethodsBlacklisted(self.info.ware_id.to_string()))
    }
}

// Iterator over {xml-comment, xml-node}, all other ignored
struct CommentTagIter<'a, 'input: 'a, T: Iterator<Item=roxmltree::Node<'a, 'input>>> {
    neighbours: T,
}
impl<'a, 'input: 'a, T: Iterator<Item=roxmltree::Node<'a, 'input>>> Iterator for CommentTagIter<'a, 'input, T> {
    type Item = (roxmltree::Node<'a, 'input>, roxmltree::Node<'a, 'input>);
    fn next(&mut self) -> Option<Self::Item> {
        // searching for [comment, element] or [comment, <text>, element]
        let mut current = self.neighbours.next()?;
        loop {
            if current.is_comment() {
                let mut next = self.neighbours.next()?;
                if next.is_element() {
                    return Some((current, next));
                }
                else if next.is_text() {
                    next = self.neighbours.next()?;
                    if next.is_element() {
                        return Some((current, next));
                    }
                }
                current = next;
            }
            else {
                current = self.neighbours.next()?;
            }
        }
    }
}
fn try_name_to_unicode_id (input: String) -> String {
    // this function translates translation id names to ProjectFluent keys
    // (unicode id)
    match input.as_str() {
        // from 08.dat comments at <languages> tag
        "English"             => "en".into(), // english
        "German"              => "de".into(), // german
        "French"              => "fr".into(), // french
        "Italian"             => "it".into(), // italian
        "Russian"             => "ru".into(), // russian
        "Spanish"             => "sp".into(), // spanish
        "Portuguese (Brazil)" => "pt".into(), // portuguese (brazil)
        "Polish"              => "pl".into(), // polish
        "Simplified Chinese"  => "ch".into(), // simplified chinese
        "Traditional Chinese" => "ch".into(), // traditional chinese
                                              // ( sorry, no idea how
                                              //   Unicode marks that )
        "Korean"              => "kr".into(), // korean
        "Japanese"            => "jp".into(), // japanese
        _                     => input,
    }
}
fn collect_lang_from_pairs<'a, 'input: 'a, It: Iterator<Item=roxmltree::Node<'a, 'input>>> (neighbours: It) -> Result<Vec<(String, String)>, DataError> {
    // adapted for (2023-05 version)
    // ```
    // <languages>
    //   <!-- English -->
    //   <language id="44" name="English" voice="true" warning="A restart is required for changes to take effect!" />
    //   <!-- German -->
    //   <language id="49" name="Deutsch" voice="true" warning="Damit die ц└nderung wirksam wird, ist ein Neustart notwendig!" />
    //   <!-- French -->
    //   <language id="33" name="Franц╖ais" voice="true" warning="Un redц╘marrage est nц╘cessaire pour la prise en compte des changements !" />
    // </languages>
    // ```
    // As you can see, comments are reliable, names ("Franц╖ais") are not
    let mut result: Vec<(String,String)> = Vec::new();
    //let mut overwritten_warnings = Vec::new();
    for (comment, node) in (CommentTagIter {neighbours}).filter(|(_, node)| node.tag_name().name() == "language") {
        let key = try_name_to_unicode_id(comment.text().unwrap().trim().into());
        if let Some(_) = result.iter().find_map(|(existing_key, existing_value)| (existing_key.as_str() == key).then_some(existing_value)) {
            if key != "ch" {
                // I have no idea how to handle Chinese & Simplified Chinese
                //overwritten_warnings.push((key.clone(), existing_value.clone()));
                return Err(DataError::DuplicateValue(key, format!("{:?}", node)));
            }
            else {
                continue;
            }
        }
        else {
            result.push((key.clone(), node.attribute("id").ok_or(DataError::AttributeNotFound("id", format!("{:?}", node)))?.to_string()));
        }
    }
    Ok(result)
}
//fn print_children (node: roxmltree::Node) {
//    println!("# {:?} [", node);
//    for n in node.children() {
//        println!("{:?}, ", n);
//    }
//    println!("]");
//}
fn find_langs_rec (node: roxmltree::Node) -> Result<Vec<(String, String)>, DataError> {
    //print_children(node);
    if let Ok(result) = collect_lang_from_pairs(node.children()) {
        Ok(result)
    }
    else {
        let mut prev_error = None;
        for node in node.children() {
            match find_langs_rec(node) {
                Ok(ok) => return Ok(ok),
                Err(err) => prev_error = Some(err),
            }
        }
        Err(prev_error.unwrap_or(DataError::NoChildren(format!("{:?}", node))))
    }
}

fn find_langs (input: &str, dbg_path: &std::path::Path) -> Result<Vec<(String, String)>, DataError> {
    let key1 = "<languages>";
    let key1_idx = input.find(key1).ok_or(DataError::SubstrNotFound(key1))?;
    let after_key1 = &input[key1_idx+key1.len()..];
    let key2 = "</languages>";
    let end_idx = key1_idx + key1.len() + after_key1.find(key2).ok_or(DataError::SubstrNotFound(key2))? + key2.len();

    let slice = &input[key1_idx..end_idx].trim();
    if slice.is_empty() {
        return Err(DataError::EmptyResultForItem("languages"));
    }
    let doc = roxmltree::Document::parse(slice).map_err(|e| DataError::InvalidXml(dbg_path.to_path_buf(), e))?;

    let result = find_langs_rec(doc.root_element())?;
    Ok(result)
}

#[derive(Debug, Clone)]
struct Wares {
    id_to_dsc: Vec<(String, SingleWare)>,
}
impl Wares {
    fn get (&self, the_key: &str) -> Result<&SingleWare, DataError> {
        self.id_to_dsc.iter().find_map(|(key, value)| if key.as_str() == the_key {Some(value)} else {None}).ok_or(DataError::UnknownWare(the_key.to_string()))
    }
    fn read_translation<'a> (node: roxmltree::Node<'a, 'static>, is_required: bool) -> Result<Option<TranslationPos>, DataError> {
        lazy_static::lazy_static! {
            static ref RE: regex::Regex = regex::Regex::new(r"^\{(?P<page>[[:digit:]]+),[[:space:]]*(?P<itemid>[[:digit:]]+)}$").unwrap();
        }
        let atr_key = "name";
        let content = node.attribute(atr_key).ok_or(DataError::AttributeNotFound(atr_key, format!("{:?}", node)))?.trim();
        //let capture = RE.captures(content).ok_or(DataError::AttributeBadValue(atr_key, format!("{:?}", node), "{pagenum:stringid}, e.g. {20201,402}"))?;
        let capture
            = match RE.captures(content) {
                  Some(capture) => capture,
                  // found `name="(TEMP)nividiumgems"`
                  None if is_required => return Err(DataError::AttributeBadValue(atr_key, format!("{:?}", node), "{pagenum:stringid}, e.g. {20201,402}")),
                  _ => return Ok(None),
              };
        let page = capture.name("page").unwrap().as_str().into();
        let id   = capture.name("itemid").unwrap().as_str().into();
        Ok(Some(TranslationPos{page, id}))
    }
    fn load_from_xml (xml: &roxmltree::Document<'static>) -> Result<Self, DataError> {
        let mut id_to_dsc = Vec::new();
        for node in xml.root_element().children().filter(|n| n.has_tag_name("ware")) {
            let ware_id_key = "id";
            let ware_id = mygetatr(node, ware_id_key)?.to_string();
            let transport = mygetatr(node, "transport")?.to_string();
            let (price_min, price_max) = {
                let price_node = myfindchildtag(node, "price")?;
                ( mygetatrparsed(price_node, "min")?,
                  mygetatrparsed(price_node, "max")? )
            };
            let ware_id_key = ware_id.clone();
            let info = SingleWareInfo {
                           ware_id,
                           price_min,
                           price_max,
                           transport,
                           };
            let translation = Self::read_translation(node, false)?;
            let mut production_methods = Vec::new();
            for prod_node in node.children().filter(|n| n.has_tag_name("production")) {
                let method = mygetatr(prod_node, "method")?.to_string();
                if production_methods.iter().find(|(m,_)| m == &method).is_some() {
                    return Err(DataError::DuplicateValue(
                               method,
                               format!("{:?}", node))
                               );
                }
                let cicle_seconds = mygetatrparsed(prod_node, "time")?;
                let wares_per_cicle = mygetatrparsed(prod_node, "amount")?;

                let wares_dependencies
                    = 'wb: {
                        if let Ok(dependencies) = myfindchildtag(prod_node, "primary") {
                            let wares_dependencies
                                = dependencies.children().filter(
                                     |n|
                                     n.has_tag_name("ware")
                                     )
                                  .try_fold(
                                      Vec::new(),
                                      |mut acc, node| {
                                          let key = mygetatr(node, "ware")?.to_string();
                                          if acc.iter().find(|(k,_)| k == &key).is_some() {
                                              return Err(DataError::DuplicateValue(
                                                          key,
                                                          format!("{:?}", dependencies)
                                                          ));
                                          }
                                          acc.push((key, mygetatrparsed(node, "amount")?));
                                          Ok(acc)
                                      });
                            if let Ok(wares_dependencies) = wares_dependencies {
                                break 'wb wares_dependencies;
                            }
                        }
                        Vec::new()
                    };
                //let dependencies = myfindchildtag(prod_node, "primary")?;
                //let wares_dependencies
                //    = dependencies.children().filter(
                //         |n|
                //         n.has_tag_name("ware")
                //         )
                //      .try_fold(
                //          Vec::new(),
                //          |mut acc, node| {
                //              let key = mygetatr(node, "ware")?.to_string();
                //              if acc.iter().find(|(k,_)| k == &key).is_some() {
                //                  return Err(DataError::DuplicateValue(
                //                              key,
                //                              format!("{:?}", dependencies)
                //                              ));
                //              }
                //              acc.push((key, mygetatrparsed(node, "amount")?));
                //              Ok(acc)
                //          })?;
                let new_key = method.clone();
                let translation = Self::read_translation(prod_node, true)?.unwrap();
                let new_value = SingleWareProduction {
                                    method,
                                    cicle_seconds,
                                    wares_per_cicle,
                                    wares_dependencies,
                                    translation,
                                    };
                production_methods.push((new_key, new_value));
            }
            let new_ware = SingleWare {
                               info,
                               production_methods,
                               translation,
                               };
            id_to_dsc.push((ware_id_key, new_ware));
        }
        Ok(Wares{id_to_dsc})
    }
    //fn gen_production_methods_list (&self) -> Vec<String> {
    //    // https://qna.habr.com/q/1289244
    //    let mut sort = topological_sort::TopologicalSort::<&String>::new();
    //    for methods_list in self.id_to_dsc.iter().map(|(_, ware)| ware.production_methods) {
    //        for (before, after) in methods_list.iter().map(|(name, _)| name).zip(methods_list.iter().skip(1).map(|(name, _)| name)) {
    //            sort.add_dependency(before, after);
    //        }
    //    }
    //    let mut result = Vec::new();
    //    while 0 != sort.len() {
    //        let new_all = sort.pop_all();
    //        if new_all.is_empty() {
    //            // cyclic dependencies
    //            return result;
    //        }
    //        result.extend(new_all.into_iter().cloned());
    //    }
    //    result
    //}
    fn load_wares_translationids_and_productionmethods_from_string (content: String, dbg_path: std::path::PathBuf) -> Result<(Self, Vec<(String, String)>), DataError> {
        let mut origin = read_xml(content, find_wares_xml_slice, "wares", &dbg_path)?;

        // now trying extract lang code maps
        let (string, doc) = origin.get_mut();
        let translations_ids = find_langs(string.as_str(), &dbg_path)?;
        let me = Self::load_from_xml(&doc[0])?;
        //let production_methods = me.gen_production_methods_list();

        Ok(( me,
             translations_ids, ))
    }
    fn load_wares_translationids_and_productionmethods (gamedir: &std::path::Path) -> Result<(Self, Vec<(String, String)>), DataError> {
        let (content, dbg_path) = load_string_file(gamedir, PRODUCTION_FILE)?;
        Self::load_wares_translationids_and_productionmethods_from_string(content, dbg_path)
    }
    fn gen_production_methods_list (&self) -> Vec<String> {
        let all_methods
            = self.id_to_dsc.iter()
                  .map(|(_, ware)| ware.production_methods.iter()
                       .map(|(_, production_method)|
                            ( &production_method
                                   .method,
                               production_method
                                   .translation.id.parse::<u32>().unwrap())
                            )
                       ).flatten()
                  .fold(
                      HashMap::new(),
                      |mut acc, (name, translation_id)| {
                          if !acc.contains_key(name) {
                              acc.insert(name.clone(), translation_id);
                          }
                          acc
                      }
                      );
        let mut all_methods = Vec::from_iter(all_methods.into_iter());
        all_methods.sort_by_key(|(_, translation_id)| *translation_id);
        let result = all_methods.iter().map(|(method, _)| method.clone()).collect();
        result
    }
}
#[derive(Debug, Clone)]
struct Fabrics<'a> {
    wares: &'a Wares,
    acc:   Vec<(String, f64, &'a SingleWareInfo, Option<(usize, &'a SingleWareProduction)>)>,
}
impl<'a> Fabrics<'a> {
    fn into_acc (self) -> Vec<(String, f64, &'a SingleWareInfo, Option<(usize, &'a SingleWareProduction)>)> {
        self.acc
    }
    fn new (wares: &'a Wares) -> Self {
        Self {wares, acc: Vec::new()}
    }
    fn find (&mut self, ware_id: &str) -> Option<&mut (String, f64, &'a SingleWareInfo, Option<(usize, &'a SingleWareProduction)>)> {
        self.acc.iter_mut().find(|(id, ..)| id == ware_id)
    }
    fn add_wares_rec (&mut self, ware_id: &str, wares_per_minute: f64, prioritylist: &[String], blacklist: &[String]) -> Result<(), DataError> {
        if 0. == wares_per_minute {
            return Ok(())
        }
        else if 0. > wares_per_minute {
            panic!("For ware_id \"{}\" got request for negative \"{}\" wares per minute count!", ware_id, wares_per_minute);
        }
        let count_added_opt
            = match self.find(ware_id) {
                  Some((_, acc_wares_per_minute, _, Some((acc_count, ware_production)))) => {
                      *acc_wares_per_minute += wares_per_minute;
                      let prev_value         = *acc_count;
                      *acc_count             = ware_production.fabrics_count_from_desired_wares_per_minute(*acc_wares_per_minute);

                      let to_add = *acc_count - prev_value;
                      if 0 == to_add {
                          None
                      }
                      else {
                        Some((*acc_count - prev_value, *ware_production))
                      }
                  },
                  Some((_, acc_wares_per_minute, _, None)) => {
                      *acc_wares_per_minute += wares_per_minute;
                      None
                  },
                  None => {
                      let ware = self.wares.get(ware_id)?;
                      let to_produce_opt
                          = if !ware.production_methods.is_empty() {
                                let ware_production = ware.find_desired_method(prioritylist, blacklist)?;
                                let count = ware_production.fabrics_count_from_desired_wares_per_minute(wares_per_minute);
                                Some((count, ware_production))
                            }
                            else {
                                None
                            };
                      self.acc.push((ware_id.to_string(), wares_per_minute, &ware.info, to_produce_opt));
                      to_produce_opt
                  },
              };
        if let Some((count_added, ware_production)) = count_added_opt {
            for (dependency_name, wares_per_minute) in ware_production.dependencies_per_minute() {
                self.add_wares_rec(dependency_name, wares_per_minute*count_added as f64, prioritylist, blacklist)?;
            }
        }
        Ok(())
    }
    fn add_wares (&mut self, ware_id: &str, wares_per_minute: f64, prioritylist: &[String], blacklist: &[String]) -> Result<(), DataError> {
        self.add_wares_rec(ware_id, wares_per_minute, prioritylist, blacklist)
    }
    fn add_fabrics (&mut self, ware_id: &str, fabrics_count: usize, prioritylist: &[String], blacklist: &[String]) -> Result<(), DataError> {
        let wares_per_minute = self.wares.get(ware_id)?.find_desired_method(prioritylist, blacklist)?.wares_per_minute() * fabrics_count as f64;
        self.add_wares(ware_id, wares_per_minute, prioritylist, blacklist)
    }
}

#[derive(Debug, Clone)]
struct Translations {
    // 'item' is ware or production method
    desired_unicode_id: String,
    unicode_id_to_item_id_to_translation: HashMap<String, HashMap<String, String>>,
}
impl Translations {
    fn set_desired_unicode_id (&mut self, desired_unicode_id: String) {
        self.desired_unicode_id = desired_unicode_id;
    }
    fn replace_if_exists (&self, item_id: &mut String) -> bool {
        if let Some(translation) = self.get(item_id) {
            *item_id = translation;
            true
        }
        else {
            false
        }
    }
    fn get (&self, item_id: &str) -> Option<String> {
        if let Some(item_to_tr) = self.unicode_id_to_item_id_to_translation.get(&self.desired_unicode_id) {
            if let Some((_, tr)) = item_to_tr.iter().find(|(item, _)| item.as_str() == item_id) {
                return Some(tr.to_string());
            }
        }
        if let Some(item_to_tr) = self.unicode_id_to_item_id_to_translation.get("en") {
            if let Some((_, tr)) = item_to_tr.iter().find(|(item, _)| item.as_str() == item_id) {
                return Some(tr.to_string());
            }
        }
        None
    }
    fn read_translation_value<'a> (xml: roxmltree::Node<'a, 'static>, node: roxmltree::Node<'a, 'static>) -> Option<String> {
        //use regex::Replacer;
        //if node.text().unwrap().starts_with("(ARG S All-round Engine Mk1") {
        //    let a = 3;
        //}
        lazy_static::lazy_static! {
            // [[:word:]]
            static ref RE_COMMENT: regex::Regex = regex::Regex::new(r"^(?P<comment>\((?:[[:word:]]|[ \-_])+\))").unwrap();
            static ref RE_REF: regex::Regex = regex::Regex::new(r"\{(?P<page>[[:digit:]]+),[[:space:]]*(?P<itemid>[[:digit:]]+)\}").unwrap();
        }
        let mut content = node.text()?.trim().to_string();
        if let Some(_) = RE_COMMENT.captures(&content) {
            content = RE_COMMENT.replace(&content, "").into_owned();
        }
        let mut is_bad = false;
        //println!("for \"{}\" capture: {:?}", content, RE_REF.captures(&content));
        content = RE_REF.replace_all(
            &content,
            |cap: &regex::Captures| {
                let page_id = cap.name("page").unwrap().as_str();
                let item_id = cap.name("itemid").unwrap().as_str();

                if let Some(page) = xml.children().find(|node| node.has_tag_name("page") && node.attribute("id").filter(|&atr| atr == page_id).is_some()) {
                    if let Some(node) = page.children().find(|node| node.has_tag_name("t") && node.attribute("id").filter(|&atr| atr == item_id).is_some()) {
                        if let Some(result) = Self::read_translation_value(xml, node) {
                            return result;
                        }
                    }
                }
                is_bad = true;
                format!("{{{},{}}}", page_id, item_id)
                //let page = xml.children().find(|node| node.has_tag_name("page") && node.attribute("id").filter(|&atr| atr == page_id).is_some())?;
                //let node = page.children().find(|node| node.has_tag_name("t") && node.attribute("id").filter(|&atr| atr == item_id).is_some())?;
                //Self::read_translation_value(xml, node).unwrap_or(format!("{{{},{}}}", page_id, item_id))
            }).into_owned();
        if is_bad {
            None
        }
        else {
            Some(content)
        }

        //if let Some(cap) = RE.captures(content) {
        //    let comment = cap.name("comment").unwrap().as_str();
        //    let page_id = cap.name("page").unwrap().as_str();
        //    let item_id = cap.name("itemid").unwrap().as_str();

        //    let page = xml.children().find(|node| node.has_tag_name("page") && node.attribute("id").filter(|&atr| atr == page_id).is_some())?;
        //    let node = page.children().find(|node| node.has_tag_name("t") && node.attribute("id").filter(|&atr| atr == item_id).is_some())?;
        //    Self::read_translation_value(xml, node)
        //}
        //else {
        //    Some(content.to_string())
        //}
    }
    fn load_from_xml_all (all_xml: &[roxmltree::Document<'static>], wares: &Wares, lang_ids: Vec<(String, String)>) -> Self {
        let mut unicode_id_to_item_id_to_translation = HashMap::new();
        for (unicode_lang_id, x4_lang_id) in lang_ids.iter() {
            let mut item_to_translation = HashMap::new();
            if let Some(lang) = all_xml.iter().find_map(
                                    |doc| {
                                        let root = doc.root_element();
                                        if root.has_tag_name("language") && root.attribute("id").filter(|&lang_id| lang_id == x4_lang_id).is_some() {
                                            Some(root)
                                        }
                                        else {
                                            None
                                        }
                                    })
            {
                for (item_id, pageno) in wares.id_to_dsc.iter().map(|(_, ware)| ware.production_methods.iter().map(|(_, method)| (&method.method, &method.translation)).chain(std::iter::once((&ware.info.ware_id, &ware.translation)).filter_map(|(id, pageno_opt)| pageno_opt.as_ref().map(|v| (id, v))))).flatten() {
                    if item_to_translation.contains_key(item_id) {
                        continue;
                    }
                    if let Some(page) = lang.children().find(|node| node.has_tag_name("page") && node.attribute("id").filter(|&id| id == pageno.page.as_str()).is_some()) {
                        if let Some(translation) = page.children().find(|node| node.has_tag_name("t") && node.attribute("id").filter(|&id| id == pageno.id.as_str()).is_some()) {
                            if let Some(new_string) = Self::read_translation_value(lang, translation) {
                                item_to_translation.insert(item_id.clone(), new_string);
                            }
                        }
                    }
                }
            }
            if !item_to_translation.is_empty() {
                unicode_id_to_item_id_to_translation.insert(unicode_lang_id.clone(), item_to_translation);
            }
        }

        Self {
            unicode_id_to_item_id_to_translation,
            desired_unicode_id: "en".to_string(),
        }
    }
    fn load_from_string (content: String, wares: &Wares, lang_ids: Vec<(String, String)>, dbg_path: std::path::PathBuf) -> Result<Self, DataError> {
        let mut origin = read_xml(content, find_translation_xml_slice, "translations", &dbg_path)?;

        // now trying extract lang code maps
        //let (string, doc) = origin.get_mut();
        let (_, doc) = origin.get_mut();
        //let translations_ids = find_langs(string.as_str(), &dbg_path)?;
        let me = Self::load_from_xml_all(&doc, wares, lang_ids);
        Ok(me)
    }
    fn load (gamedir: &std::path::Path, wares: &Wares, lang_ids: Vec<(String, String)>) -> Result<Self, DataError> {
        let (content, dbg_path) = load_string_file(gamedir, TRANSLATIONS_FILE)?;
        Self::load_from_string(content, wares, lang_ids, dbg_path)
    }
}

#[derive(Debug, Clone)]
pub enum CountsInput {
    WaresPerMinute(f64),
    Fabrics(usize),
}
impl serde::Serialize for CountsInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let string = match self {
                         Self::WaresPerMinute(v) => format!("WaresPerMinute({})", v),
                         Self::Fabrics(v) => format!("Fabrics({})", v),
                     };
        serializer.serialize_str(&string)
    }
}
impl<'de> serde::Deserialize<'de> for CountsInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        struct MyVariantVisitor;
        impl<'de> serde::de::Visitor<'de> for MyVariantVisitor {
            type Value = CountsInput;
            fn expecting (&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_fmt(format_args!("Either WaresPerMinute(f64) either Fabrics(usize)"))
            }
            fn visit_str<E: serde::de::Error> (self, v: &str) -> Result<Self::Value, E> {
                use nom::combinator::{all_consuming, map};
                use nom::branch::alt;
                use nom::bytes::complete::tag;
                use nom::sequence::delimited;

                let invalid_str = |s| E::invalid_value(serde::de::Unexpected::Str(s), &self);
                //let variant_str = v.parse().map_err(|_| invalid_str(v))?;
                let (_,ok) = all_consuming(alt((
                                 map(
                                     delimited(
                                         tag("WaresPerMinute("),
                                         nom::number::complete::float::<_,()>,
                                         tag(")")
                                         ),
                                     |f| Self::Value::WaresPerMinute(f as f64)
                                     ),
                                 map(
                                     delimited(
                                         tag("Fabrics("),
                                         nom::character::complete::u32,
                                         tag(")")
                                         ),
                                     |u| Self::Value::Fabrics(u as usize)
                                     ),
                                 )))(v).map_err(|_| invalid_str(v))?;
                Ok(ok)
            }
        }
        deserializer.deserialize_str(MyVariantVisitor)
    }
}
#[derive(Debug, Clone)]
pub enum CountsOutput {
    Produce(String, usize),
    Import,
}
impl serde::Serialize for CountsOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let string = match self {
                         Self::Produce(s, v) => format!("Produce({}, {})", s, v),
                         Self::Import => format!("Import"),
                     };
        serializer.serialize_str(&string)
    }
}
impl<'de> serde::Deserialize<'de> for CountsOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        struct MyVariantVisitor;
        impl<'de> serde::de::Visitor<'de> for MyVariantVisitor {
            type Value = CountsOutput;
            fn expecting (&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_fmt(format_args!("Either Produce(\"method\", usize) either Import"))
            }
            fn visit_str<E: serde::de::Error> (self, mut v: &str) -> Result<Self::Value, E> {
                let invalid_str = |s| E::invalid_value(serde::de::Unexpected::Str(s), &self);
                //let variant_str = v.parse().map_err(|_| invalid_str(v))?;
                if let Some((method, count)) = (|| {
                    let prefix = "Produce(";
                    if v.starts_with(prefix) && v.ends_with(")") {
                        v = &v[prefix.len()..v.len()-1];
                        if let Some(separator_pos) = v.find(',') {
                            if let Ok(count) = v[1+separator_pos..].parse() {
                                return Some((&v[..separator_pos], count))
                            }
                        }
                    }
                    None
                })() {
                    Ok(CountsOutput::Produce(method.to_string(), count))
                }
                else if v == "Import" {
                    Ok(CountsOutput::Import)
                }
                else {
                    Err(invalid_str(v))
                }
            }
        }
        deserializer.deserialize_str(MyVariantVisitor)
    }
}

#[derive(Debug, Clone)]
pub struct Data {
    wares: Wares,
    translations: Translations,
}

impl Data {
    pub fn set_desired_unicode_id (&mut self, desired_unicode_id: String) {
        self.translations.set_desired_unicode_id(desired_unicode_id);
    }
    pub fn load_data_str (wares_xml_str: String, translation_xml_str: String) -> Result<Self, DataError> {
        let (wares, lang_ids_map) = Wares::load_wares_translationids_and_productionmethods_from_string(wares_xml_str, "<local>".into())?;
        let translations = Translations::load_from_string(translation_xml_str, &wares, lang_ids_map, "<local".into())?;
        Ok(Self {
            wares,
            translations,
        })
    }
    pub fn load_data (gamedir: &std::path::Path) -> Result<Self, DataError> {
        let (wares, lang_ids_map) = Wares::load_wares_translationids_and_productionmethods(gamedir)?;
        let translations = Translations::load(gamedir, &wares, lang_ids_map)?;
        Ok(Self {
            wares,
            translations,
        })
    }
    pub fn change_default_lang (&mut self, desired_unicode_id: String) {
        self.translations.desired_unicode_id = desired_unicode_id;
    }
    pub fn gen_production_methods_list (&self) -> Vec<String> {
        self.wares.gen_production_methods_list()
    }
    pub fn gen_lang_list (&self) -> Vec<String> {
        self.translations.unicode_id_to_item_id_to_translation.keys().cloned().collect()
    }
    fn make_translation_to_item_map<It: Iterator<Item=String>> (&self, mut usr_items: It) -> Result<HashMap<String, String>, DataError> {
        let max_langs = self.translations.unicode_id_to_item_id_to_translation.len();
        let mut langs_passed = 0;
        let langs_it = self.translations.unicode_id_to_item_id_to_translation.iter().cycle();

        let mut result = HashMap::new();
        let mut usr_item = match usr_items.next() {
                               None => return Ok(result),
                               Some(usr_item) => usr_item,
                           };
        for (_, translations) in langs_it {
            if langs_passed >= max_langs {
                return Err(DataError::ReverseTranslationNotFound(usr_item));
            }
            loop {
                langs_passed += 1;
                if let Some(item_name) = translations.iter().find_map(|(item, translation)| if translation == &usr_item { Some(item) } else { None }) {
                    result.insert(usr_item, item_name.clone());

                    langs_passed = 0;

                    usr_item = loop { match usr_items.next() {
                                   None => return Ok(result),
                                   Some(item) if result.contains_key(&item) => continue,
                                   Some(item) => break item,
                               }}
                }
                if 0 != langs_passed {
                    break;
                }
            }
        }
        unreachable!()
    }
    fn check_production_prioritylist_blacklsit (&self, prioritylist: &[String], blacklist: &[String]) -> Result<(), DataError> {
        if let Some(bad_item) = prioritylist.iter().find(|p_item| blacklist.iter().find(|b_item| b_item == p_item).is_some()) {
            return Err(DataError::PriorityBlackListsIntersection(bad_item.clone()));
        }
        else if let Some(bad_item) = blacklist.iter().find(|b_item| prioritylist.iter().find(|p_item| p_item == b_item).is_some()) {
            return Err(DataError::PriorityBlackListsIntersection(bad_item.clone()));
        }
        let all_methods = self.gen_production_methods_list();
        if let Some(bad_item) = prioritylist.iter().chain(blacklist.iter()).find(|item| all_methods.iter().find(|method| method == item).is_none()) {
            return Err(DataError::UnknownProductionMethod(bad_item.clone()));
        }
        Ok(())
    }
    fn validate_and_untranslate (&self, desired_outputs: Vec<(String, CountsInput)>, prioritylist: Vec<String>, blacklist: Vec<String>) -> Result<(Vec<(String, CountsInput)>, Vec<String>, Vec<String>), DataError> {
        let usr_translation_to_item_name = self.make_translation_to_item_map(desired_outputs.iter().map(|(usr_name, _)| usr_name).cloned().chain(prioritylist.iter().cloned()).chain(blacklist.iter().cloned()))?;

        let items = desired_outputs.into_iter().map(|(usr_name, counts)| (usr_translation_to_item_name.get(&usr_name).unwrap().clone(), counts)).collect();

        let prioritylist = prioritylist.into_iter().map(|usr| usr_translation_to_item_name.get(&usr).unwrap().clone()).collect::<Vec<_>>();
        let blacklist = blacklist.into_iter().map(|usr| usr_translation_to_item_name.get(&usr).unwrap().clone()).collect::<Vec<_>>();
        self.check_production_prioritylist_blacklsit(&prioritylist, &blacklist)?;

        Ok((items, prioritylist, blacklist))
    }
    pub fn calc_required_fabric_counts (&self, desired_outputs: Vec<(String, CountsInput)>, prioritylist: Vec<String>, blacklist: Vec<String>) -> Result<Vec<(String, f64, CountsOutput, SingleWare)>, DataError> {
        let (desired_outputs, prioritylist, blacklist) = self.validate_and_untranslate(desired_outputs, prioritylist, blacklist)?;

        let mut fabrics = Fabrics::new(&self.wares);
        for (ware, desired_count) in desired_outputs {
            match desired_count {
                CountsInput::Fabrics(fabrics_count) => fabrics.add_fabrics(&ware, fabrics_count, &prioritylist, &blacklist)?,
                CountsInput::WaresPerMinute(wares_per_minute) => fabrics.add_wares(&ware, wares_per_minute, &prioritylist, &blacklist)?,
            }
        }
        let acc = fabrics.into_acc();
        let mut result
            = acc.into_iter()
              .map(
                  |(ware, wares_per_minute, ware_info, production_opt)| (
                      ware,
                      wares_per_minute,
                      production_opt.map_or(
                          CountsOutput::Import,
                          |(fabrics_count, ware_production)|
                            CountsOutput::Produce(
                                ware_production.method.clone(),
                                fabrics_count
                                )
                          ),
                      self.wares.get(&ware_info.ware_id).unwrap().clone()
                  )).collect::<Vec<_>>();

        result.iter_mut()
            .for_each(
                |(ware_id, _, production_opt, _)| {
                    self.translations.replace_if_exists(ware_id);
                    if let CountsOutput::Produce(method, _) = production_opt {
                        self.translations.replace_if_exists(method);
                    }
                });
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

const WARES1: &'static str =  r#"
<!-- line 195751 -->
<languages>
  <!-- English -->
  <language id="44" name="English" voice="true" warning="A restart is required for changes to take effect!" />
  <!-- German -->
  <language id="49" name="Deutsch" voice="true" warning="Damit die ц└nderung wirksam wird, ist ein Neustart notwendig!" />
  <!-- French -->
  <language id="33" name="Franц╖ais" voice="true" warning="Un redц╘marrage est nц╘cessaire pour la prise en compte des changements !" />
  <!-- Italian -->
  <language id="39" name="Italiano" warning="I cambiamenti saranno effettivi al prossimo riavvio!" />
  <!-- Russian -->
  <language id="7" name="п═я┐я│я│п╨п╦п╧" voice="true" warning="п÷п╣я─п╣п╥п╟пЁя─я┐п╥п╦я┌п╣ п╦пЁя─я┐, я┤я┌п╬п╠я▀ п╦п╥п╪п╣п╫п╣п╫п╦я▐ п╡я│я┌я┐п©п╦п╩п╦ п╡ я│п╦п╩я┐!" />
  <!-- Spanish -->
  <language id="34" name="Espaц╠ol" warning="б║Es necesario reiniciar para que los cambios surtan efecto!" />
  <!-- Portuguese (Brazil) -->
  <language id="55" name="Portuguц╙s (Brasil)" warning="Para que as mudanц╖as funcionem, ц╘ preciso reiniciar!" />
  <!-- Polish -->
  <language id="48" name="Polski" warning="Wymagane jest ponowne uruchomienie, aby zmiany zadziaе┌aе┌y!" />
  <!-- Simplified Chinese -->
  <language id="86" name="Г╝─Д╫⌠Д╦╜Ф√┤" font="csfont" displaytimefactor="5.0" warning="И°─Х╕│И┤█Ф√╟Е░╞Е┼╗Д╩╔Д╫©Ф⌡╢Ф■╧Г■÷Ф∙┬О╪│" />
  <!-- Traditional Chinese -->
  <language id="88" name="Г╧│И╚■Д╦╜Ф√┤" font="ctfont" displaytimefactor="5.0" warning="И°─Х╕│И┤█Ф√╟Е∙÷Е▀∙Д╩╔Д╫©Ф⌡╢Ф■╧Г■÷Ф∙┬О╪│" />
  <!-- Korean -->
  <language id="82" name="М∙°Й╣╜Л√╢" font="kofont" displaytimefactor="5.0" warning="КЁ─Й╡╫К┌╢Л ╘Л²╢ Л═│Л ╘К░≤Й╦╟ Л°└М∙╢Л└°К┼■ Л┐┬К║° Л▀°Л·▒М∙≤Л┘■Л∙╪ М∙╘К▀┬К▀╓." />
  <!-- Japanese -->
  <language id="81" name="Ф≈╔Ф°╛Х╙·" font="jfont" displaytimefactor="5.0" warning="Е╓┴Ф⌡╢Ц┌▓Ф°┴Е┼╧Ц│╚Ц│≥Ц┌▀Ц│╚Ц│╞Е├█Х╣╥Е▀∙Ц│≈Ц│╕Ц│▐Ц│═Ц│∙Ц│└О╪│" />
  <!-- Czech (not supported) -->
  <!-- <language id="42" name="д█eе║tina" /> -->
</languages>

<!-- line 287816 -->
<wares>
  <production>
    <method id="argon" name="{20206,201}">
      <default race="argon" />
    </method>
    <method id="default" name="{20206,101}" />
    <method id="paranid" name="{20206,301}">
      <default race="paranid" />
    </method>
    <method id="processing" name="{20206,1301}" tags="noplayerbuild recycling" />
    <method id="recycling" name="{20206,1101}" tags="noplayerbuild recycling" />
    <method id="research" name="{20206,501}" tags="noplayerbuild" />
    <method id="teladi" name="{20206,401}">
      <default race="teladi" />
    </method>
    <method id="xenon" name="{20206,601}" tags="noplayerbuild">
      <default race="xenon" />
    </method>
  </production>
  <defaults id="default" name="default" transport="container" volume="1" tags="container">
    <price min="1" average="1" max="1" />
    <production time="10" amount="1" method="default" name="{20206,101}">
      <effects>
        <effect type="efficiency" product="1" />
      </effects>
    </production>
    <container ref="sm_gen_pickup_container_01_macro" />
    <icon active="ware_default" video="ware_noicon_macro" />
  </defaults>

  <ware id="energycells" name="{20201,701}" description="{20201,702}" factoryname="{20201,704}" group="energy" transport="container" volume="1" tags="container economy stationbuilding">
    <price min="10" average="16" max="22" />
    <production time="60" amount="175" method="default" name="{20206,101}">
      <effects>
        <effect type="sunlight" product="1" />
        <effect type="work" product="0.43" />
      </effects>
    </production>
    <icon active="ware_energycells" video="ware_energycells_macro" />
  </ware>
  <ware id="microchips" name="{20201,2201}" description="{20201,2202}" factoryname="{20201,2204}" group="hightech" transport="container" volume="22" tags="container economy">
    <price min="805" average="948" max="1090" />
    <production time="600" amount="72" method="default" name="{20206,101}">
      <primary>
        <ware ware="energycells" amount="50" />
        <ware ware="siliconwafers" amount="200" />
      </primary>
      <effects>
        <effect type="work" product="0.36" />
      </effects>
    </production>
    <icon active="ware_microchips" video="ware_microchips_macro" />
  </ware>
  <ware id="silicon" name="{20201,3501}" description="{20201,3502}" factoryname="{20201,3504}" group="minerals" transport="solid" volume="10" tags="economy minable mineral solid">
    <price min="111" average="130" max="150" />
    <container ref="sm_gen_pickup_solid_01_macro" />
    <icon active="ware_silicon" video="ware_silicon_macro" />
  </ware>
  <ware id="siliconwafers" name="{20201,3601}" description="{20201,3602}" factoryname="{20201,3604}" group="refined" transport="container" volume="18" tags="container economy">
    <price min="180" average="299" max="419" />
    <production time="180" amount="107" method="default" name="{20206,101}">
      <primary>
        <ware ware="energycells" amount="90" />
        <ware ware="silicon" amount="240" />
      </primary>
      <effects>
        <effect type="work" product="0.37" />
      </effects>
    </production>
    <icon active="ware_siliconwafers" video="ware_siliconwafers_macro" />
  </ware>
</wares>
"#;

const TRANSLATIONS1: &'static str = r#"
<!-- line 307812 -->
<language id="44">
  <!-- line 355941 -->
  <page id="20201" title="Wares" descr="Names and descriptions of wares" voice="yes">
    <t id="100">(*PRODUCED WARES*)</t>
    <t id="701">Energy Cells</t>
    <t id="702">Contrary to common belief, Energy Cells are not simply glorified batteries; actually, they are sophisticated bio-chemical \(or bio-mechanical, depending on technology\) devices capable of storing energy near or at 100% efficiency.</t>
    <t id="704">Solar Power Plant</t>
    <t id="2201">Microchips</t>
    <t id="2202">Used in a wide variety of equipment parts, micro-chips are produced using silicon wafers, which, while fragile, allows them to conduct at a much higher rate. This, in turn, allows far better processing in the equipment that uses the micro-chips, which includes many advanced electronics and components.</t>
    <t id="2204">Microchip Factory</t>
    <t id="3501">Silicon</t>
    <t id="3502">Silicon, required for the production of the most common types of silicon wafers, is usually mined or harvested from asteroids or other uninhabited celestial bodies.</t>
    <t id="3504">Silicon Mine</t>
    <t id="3601">Silicon Wafers</t>
    <t id="3602">If a technology requires any kind of chip, it is highly likely that is uses silicon wafers. Light, efficient and cheap to produce, these wafers are usually layered or constructed in hexagonal meshes to allow for quick transfer of data across a component.</t>
    <t id="3604">Silicon Refinery</t>
  </page>

  <!-- line 294946 -->
  <page id="20202" title="Races" descr="Names and descriptions of races" voice="yes">
    <t id="101">Argon</t>
    <t id="102">The descendents of Terran colonists stranded from Earth centuries ago, the Argon became their own thriving civilisation covering a great many systems and forging relations with several alien races. Throughout their short history the Argon Federation has been plagued by war, notably with the Xenon. Their greatest challenge however came from the unlikely source of the reconnected Terrans of Earth where they were plunged into the costly Terran Conflict.</t>
    <t id="201">Boron</t>
    <t id="202">The predominantly peaceful Boron are aquatic life-forms from the planet Nishala. While initially pacifist, the discovery of their world by the Split forced them to invent defences and adapt to war. Enjoying a close relationship with the Argon, the Boron remain a wise and measured people.</t>
    <t id="301">Split</t>
    <t id="302">The aggressive Split live in a society constantly changing leadership where challenging factions rise up to impose a new Patriarch. Their short temper and fiery disposition puts them at odds with other races which has sometimes lead to war, notably with the Boron and Argon.</t>
    <t id="401">Paranid</t>
    <t id="402">The physically imposing Paranid are often regarded as arrogant by several races which usually stems from their exceptional mathematic skills and religious fervour. Allied with the Split and distrusting of the Argon, the Paranid have been in several conflicts where they use their technological prowess and multilevel thinking to gain tactical advantages.</t>
    <t id="501">Teladi</t>
    <t id="502">The lizard-like Teladi are one of the founding members of the Community of Planets and have a natural affinity towards business and the accumulation of profit. They enjoy favourable relations with other races although some find their drive for profit disconcerting. Their long lifespan gives them a unique view of the Jump Gate shutdown, as does their previous experience being cut off from their home system of Ianamus Zura.</t>
    <t id="601">Xenon</t>
    <t id="602">The Xenon are a mechanical race resulting from past Terran terraformer ships which eventually evolved intelligence. A constant threat in many areas of the galaxy, it is thought that the Jump Gate shutdown may stem their movements but given their disregard of time it is possible they may simply travel between stars. The Xenon have no known allies and communication with them is often relegated to folklore.</t>
    <t id="701">Terran</t>
    <t id="702">The Terrans of the Solar System have a long history of spaceflight and exploring the Jump Gate network. After the events of the Terraformers over Earth, the Terrans severed their contact with the rest of the galaxy and had several centuries of rebuilding and advancement in isolation. Their brief return led to the Terran Conflict which preceded the mass disconnection of Jump Gates. It is unknown if the war precipitated this event.</t>
    <t id="901">Kha'ak</t>
    <t id="902">Thought to have been wiped out during Operation Final Fury, very little is known about the Kha'ak other than they seem to be an insectile hive race hell-bent on the destruction of all those that share the Jump Gate network. As a hive race, it is suspected that individual intelligence gives way to a communal or caste mentality, but very little research into the species was completed before Operation Final Fury took place.</t>
  </page>

  <!-- line 356957 -->
  <page id="20206" title="Ware Production Methods" descr="Names and descriptions of production methods used to produce wares" voice="yes">
    <t id="101">Universal</t>
    <t id="201">(Argon){20202,101}</t>
    <t id="301">(Paranid){20202,401}</t>
    <t id="401">(Teladi){20202,501}</t>
    <t id="501">Research</t>
    <t id="601">(Xenon){20202,601}</t>
    <t id="701">(Split){20202,301}</t>
    <t id="801">(Boron){20202,201}</t>
    <t id="901">(Terran){20202,701}</t>
    <t id="1001">Default</t>
  </page>
</language>
"#;

const WARES2: &'static str = r#"
<!-- line 195751 -->
<languages>
  <!-- English -->
  <language id="44" name="English" voice="true" warning="A restart is required for changes to take effect!" />
  <!-- German -->
  <language id="49" name="Deutsch" voice="true" warning="Damit die ц└nderung wirksam wird, ist ein Neustart notwendig!" />
  <!-- French -->
  <language id="33" name="Franц╖ais" voice="true" warning="Un redц╘marrage est nц╘cessaire pour la prise en compte des changements !" />
  <!-- Italian -->
  <language id="39" name="Italiano" warning="I cambiamenti saranno effettivi al prossimo riavvio!" />
  <!-- Russian -->
  <language id="7" name="п═я┐я│я│п╨п╦п╧" voice="true" warning="п÷п╣я─п╣п╥п╟пЁя─я┐п╥п╦я┌п╣ п╦пЁя─я┐, я┤я┌п╬п╠я▀ п╦п╥п╪п╣п╫п╣п╫п╦я▐ п╡я│я┌я┐п©п╦п╩п╦ п╡ я│п╦п╩я┐!" />
  <!-- Spanish -->
  <language id="34" name="Espaц╠ol" warning="б║Es necesario reiniciar para que los cambios surtan efecto!" />
  <!-- Portuguese (Brazil) -->
  <language id="55" name="Portuguц╙s (Brasil)" warning="Para que as mudanц╖as funcionem, ц╘ preciso reiniciar!" />
  <!-- Polish -->
  <language id="48" name="Polski" warning="Wymagane jest ponowne uruchomienie, aby zmiany zadziaе┌aе┌y!" />
  <!-- Simplified Chinese -->
  <language id="86" name="Г╝─Д╫⌠Д╦╜Ф√┤" font="csfont" displaytimefactor="5.0" warning="И°─Х╕│И┤█Ф√╟Е░╞Е┼╗Д╩╔Д╫©Ф⌡╢Ф■╧Г■÷Ф∙┬О╪│" />
  <!-- Traditional Chinese -->
  <language id="88" name="Г╧│И╚■Д╦╜Ф√┤" font="ctfont" displaytimefactor="5.0" warning="И°─Х╕│И┤█Ф√╟Е∙÷Е▀∙Д╩╔Д╫©Ф⌡╢Ф■╧Г■÷Ф∙┬О╪│" />
  <!-- Korean -->
  <language id="82" name="М∙°Й╣╜Л√╢" font="kofont" displaytimefactor="5.0" warning="КЁ─Й╡╫К┌╢Л ╘Л²╢ Л═│Л ╘К░≤Й╦╟ Л°└М∙╢Л└°К┼■ Л┐┬К║° Л▀°Л·▒М∙≤Л┘■Л∙╪ М∙╘К▀┬К▀╓." />
  <!-- Japanese -->
  <language id="81" name="Ф≈╔Ф°╛Х╙·" font="jfont" displaytimefactor="5.0" warning="Е╓┴Ф⌡╢Ц┌▓Ф°┴Е┼╧Ц│╚Ц│≥Ц┌▀Ц│╚Ц│╞Е├█Х╣╥Е▀∙Ц│≈Ц│╕Ц│▐Ц│═Ц│∙Ц│└О╪│" />
  <!-- Czech (not supported) -->
  <!-- <language id="42" name="д█eе║tina" /> -->
</languages>

<!-- line 287816 -->
<wares>
  <production>
    <method id="argon" name="{20206,201}">
      <default race="argon" />
    </method>
    <method id="default" name="{20206,101}" />
    <method id="paranid" name="{20206,301}">
      <default race="paranid" />
    </method>
    <method id="processing" name="{20206,1301}" tags="noplayerbuild recycling" />
    <method id="recycling" name="{20206,1101}" tags="noplayerbuild recycling" />
    <method id="research" name="{20206,501}" tags="noplayerbuild" />
    <method id="teladi" name="{20206,401}">
      <default race="teladi" />
    </method>
    <method id="xenon" name="{20206,601}" tags="noplayerbuild">
      <default race="xenon" />
    </method>
  </production>

  <ware id="engine_arg_s_allround_01_mk1" name="{20107,1004}" description="{20107,1002}" group="engines" transport="equipment" volume="1" tags="engine equipment">
    <price min="5526" average="6140" max="6754" />
    <production time="10" amount="1" method="default" name="{20206,101}">
      <primary>
        <ware ware="energycells" amount="10" />
        <ware ware="engineparts" amount="4" />
      </primary>
    </production>
    <component ref="engine_arg_s_allround_01_mk1_macro" />
    <restriction licence="generaluseequipment" />
    <use threshold="0" />
    <owner faction="alliance" />
    <owner faction="antigone" />
    <owner faction="argon" />
    <owner faction="buccaneers" />
    <owner faction="hatikvah" />
  </ware>
  <ware id="energycells" name="{20201,701}" description="{20201,702}" factoryname="{20201,704}" group="energy" transport="container" volume="1" tags="container economy stationbuilding">
    <price min="10" average="16" max="22" />
    <production time="60" amount="175" method="default" name="{20206,101}">
      <effects>
        <effect type="sunlight" product="1" />
        <effect type="work" product="0.43" />
      </effects>
    </production>
    <icon active="ware_energycells" video="ware_energycells_macro" />
  </ware>
  <ware id="engineparts" name="{20201,801}" description="{20201,802}" factoryname="{20201,804}" group="hightech" transport="container" volume="15" tags="container economy">
    <price min="128" average="182" max="237" />
    <production time="900" amount="208" method="default" name="{20206,101}">
      <primary>
        <ware ware="antimattercells" amount="80" />
        <ware ware="energycells" amount="60" />
        <ware ware="refinedmetals" amount="96" />
      </primary>
      <effects>
        <effect type="work" product="0.47" />
      </effects>
    </production>
    <production time="900" amount="208" method="teladi" name="{20206,401}">
      <primary>
        <ware ware="antimattercells" amount="80" />
        <ware ware="energycells" amount="60" />
        <ware ware="teladianium" amount="70" />
      </primary>
      <effects>
        <effect type="work" product="0.47" />
      </effects>
    </production>
    <icon active="ware_engineparts" video="ware_engineparts_macro" />
  </ware>
  <ware id="antimattercells" name="{20201,201}" description="{20201,202}" factoryname="{20201,204}" group="refined" transport="container" volume="18" tags="container economy">
    <price min="121" average="202" max="282" />
    <production time="120" amount="99" method="default" name="{20206,101}">
      <primary>
        <ware ware="energycells" amount="100" />
        <ware ware="hydrogen" amount="320" />
      </primary>
      <effects>
        <effect type="work" product="0.35" />
      </effects>
    </production>
    <icon active="ware_antimattercells" video="ware_antimattercells_macro" />
  </ware>
  <ware id="hydrogen" name="{20201,1301}" description="{20201,1302}" factoryname="{20201,1304}" group="gases" transport="liquid" volume="6" tags="economy gas liquid minable">
    <price min="49" average="58" max="67" />
    <container ref="sm_gen_pickup_liquid_01_macro" />
    <icon active="ware_hydrogen" video="ware_hydrogen_macro" />
  </ware>
  <ware id="refinedmetals" name="{20201,3201}" description="{20201,3202}" factoryname="{20201,3204}" group="refined" transport="container" volume="14" tags="container economy">
    <price min="89" average="148" max="207" />
    <production time="150" amount="88" method="default" name="{20206,101}">
      <primary>
        <ware ware="energycells" amount="90" />
        <ware ware="ore" amount="240" />
      </primary>
      <effects>
        <effect type="work" product="0.43" />
      </effects>
    </production>
    <icon active="ware_refinedmetals" video="ware_refinedmetals_macro" />
  </ware>
  <ware id="ore" name="{20201,2701}" description="{20201,2702}" factoryname="{20201,2704}" group="minerals" transport="solid" volume="10" tags="economy minable mineral solid">
    <price min="43" average="50" max="58" />
    <container ref="sm_gen_pickup_solid_01_macro" />
    <icon active="ware_ore" video="ware_ore_macro" />
  </ware>
</wares>
"#;

const TRANSLATIONS2: &'static str = r#"
<!-- line 307812 -->
<language id="44">
  <!-- line 355941 -->
  <page id="20201" title="Wares" descr="Names and descriptions of wares" voice="yes">
    <t id="100">(*PRODUCED WARES*)</t>
    <t id="201">Antimatter Cells</t>
    <t id="202">Highly advanced magnetic storage devices that carry antimatter. Due to the effect of Hawking radiation and their being self-powered, antimatter cells cannot store antimatter indefinitely. They are produced and filled using refined hydrogen and primarily used in the production of engine parts, and also can be miniaturised to be used in claytronics.</t>
    <t id="204">Antimatter Cell Factory</t>
    <t id="701">Energy Cells</t>
    <t id="702">Contrary to common belief, Energy Cells are not simply glorified batteries; actually, they are sophisticated bio-chemical \(or bio-mechanical, depending on technology\) devices capable of storing energy near or at 100% efficiency.</t>
    <t id="704">Solar Power Plant</t>
    <t id="801">Engine Parts</t>
    <t id="802">Comprised of a number of different components that make up ship engines, engine parts are delivered straight to the end customer, most commonly shipyards and equipment docks, who then use them themselves to produce or repair engines. While naturally engine parts are a very necessary resource across the entire Jump Gate network, the ability to produce and repair engines on demand, instead of requiring an entirely separate production step for each, has greatly streamlined the universal economy.</t>
    <t id="804">Engine Part Factory</t>
    <t id="1301">Hydrogen</t>
    <t id="1302">Historically, Hydrogen has been used mainly in H-fusion generators. These days however, with the rise of sustainable M/AM mass conversion, Hydrogen is routinely converted into anti-Hydrogen for use in Antimatter Cells.</t>
    <t id="1304">Hydrogen Extractor</t>
    <t id="3201">Refined Metals</t>
    <t id="3202">Refined from ore found in countless asteroids across the Jump Gate network, these refined metals are cheap to produce and easy to reinforce, making them perfect for use in constructing all kinds of Hull Parts, not just for ships and stations, but also for smaller components that used across all of space.</t>
    <t id="3204">Ore Refinery</t>
    <t id="2701">Ore</t>
    <t id="2702">Today ore tends not to be mined on habitable worlds, but harvested from other celestial bodies, mainly asteroids. As could be expected, Ore must always be refined to be of any use.</t>
    <t id="2704">Ore Mine</t>
  </page>

  <!-- line 353566 -->
  <page id="20107" title="Engines Thrusters" descr="Names and descriptions of ship engines" voice="no">
    <t id="1000">(*Small Engines*)</t>
    <t id="1001">All-round Engine</t>
    <t id="1002">{20107,101}{20107,203}{20107,301}{20107,404}</t>
    <t id="1003">All-round</t>
    <t id="1004">(ARG S All-round Engine Mk1){20202,103} {20111,5011} {20107,1001} {20111,101}</t>
  </page>

  <!-- line 35299 -->
  <page id="20111" title="Object Variations" descr="Names and descriptions of object variations" voice="yes">
    <t id="101">Mk1</t>
    <t id="5011">(small)S</t>
  </page>

  <!-- line 294946 -->
  <page id="20202" title="Races" descr="Names and descriptions of races" voice="yes">
    <t id="101">Argon</t>
    <t id="102">The descendents of Terran colonists stranded from Earth centuries ago, the Argon became their own thriving civilisation covering a great many systems and forging relations with several alien races. Throughout their short history the Argon Federation has been plagued by war, notably with the Xenon. Their greatest challenge however came from the unlikely source of the reconnected Terrans of Earth where they were plunged into the costly Terran Conflict.</t>
    <t id="103">ARG</t>
    <t id="201">Boron</t>
    <t id="202">The predominantly peaceful Boron are aquatic life-forms from the planet Nishala. While initially pacifist, the discovery of their world by the Split forced them to invent defences and adapt to war. Enjoying a close relationship with the Argon, the Boron remain a wise and measured people.</t>
    <t id="301">Split</t>
    <t id="302">The aggressive Split live in a society constantly changing leadership where challenging factions rise up to impose a new Patriarch. Their short temper and fiery disposition puts them at odds with other races which has sometimes lead to war, notably with the Boron and Argon.</t>
    <t id="401">Paranid</t>
    <t id="402">The physically imposing Paranid are often regarded as arrogant by several races which usually stems from their exceptional mathematic skills and religious fervour. Allied with the Split and distrusting of the Argon, the Paranid have been in several conflicts where they use their technological prowess and multilevel thinking to gain tactical advantages.</t>
    <t id="501">Teladi</t>
    <t id="502">The lizard-like Teladi are one of the founding members of the Community of Planets and have a natural affinity towards business and the accumulation of profit. They enjoy favourable relations with other races although some find their drive for profit disconcerting. Their long lifespan gives them a unique view of the Jump Gate shutdown, as does their previous experience being cut off from their home system of Ianamus Zura.</t>
    <t id="601">Xenon</t>
    <t id="602">The Xenon are a mechanical race resulting from past Terran terraformer ships which eventually evolved intelligence. A constant threat in many areas of the galaxy, it is thought that the Jump Gate shutdown may stem their movements but given their disregard of time it is possible they may simply travel between stars. The Xenon have no known allies and communication with them is often relegated to folklore.</t>
    <t id="701">Terran</t>
    <t id="702">The Terrans of the Solar System have a long history of spaceflight and exploring the Jump Gate network. After the events of the Terraformers over Earth, the Terrans severed their contact with the rest of the galaxy and had several centuries of rebuilding and advancement in isolation. Their brief return led to the Terran Conflict which preceded the mass disconnection of Jump Gates. It is unknown if the war precipitated this event.</t>
    <t id="901">Kha'ak</t>
    <t id="902">Thought to have been wiped out during Operation Final Fury, very little is known about the Kha'ak other than they seem to be an insectile hive race hell-bent on the destruction of all those that share the Jump Gate network. As a hive race, it is suspected that individual intelligence gives way to a communal or caste mentality, but very little research into the species was completed before Operation Final Fury took place.</t>
  </page>

  <!-- line 356957 -->
  <page id="20206" title="Ware Production Methods" descr="Names and descriptions of production methods used to produce wares" voice="yes">
    <t id="101">Universal</t>
    <t id="201">(Argon){20202,101}</t>
    <t id="301">(Paranid){20202,401}</t>
    <t id="401">(Teladi){20202,501}</t>
    <t id="501">Research</t>
    <t id="601">(Xenon){20202,601}</t>
    <t id="701">(Split){20202,301}</t>
    <t id="801">(Boron){20202,201}</t>
    <t id="901">(Terran){20202,701}</t>
    <t id="1001">Default</t>
  </page>
</language>
"#;

    #[test]
    fn test_wares_parsing () {
        let content = WARES1.to_string();
        let translations = TRANSLATIONS1.to_string();
        let (wares, langs) = Wares::load_wares_translationids_and_productionmethods_from_string(content, "local".into()).unwrap();
        println!("## Done Wares = {:?}", wares);
        let translations = Translations::load_from_string(translations, &wares, langs, "local".into()).unwrap();
        println!("## Done Translations = {:?}", translations);
        let data = Data{wares, translations};
        let calced = data.calc_required_fabric_counts(vec![("Microchips".to_string(), CountsInput::WaresPerMinute(36f64))], Vec::new(), Vec::new()).unwrap();
        println!("## Calced = {:?}", calced);
    }
    #[test]
    fn test_translations_parsing () {
        let content = WARES2.to_string();
        let translations = TRANSLATIONS2.to_string();
        let (wares, langs) = Wares::load_wares_translationids_and_productionmethods_from_string(content, "local".into()).unwrap();
        println!("## Done Wares = {:?}", wares);
        let translations = Translations::load_from_string(translations, &wares, langs, "local".into()).unwrap();
        println!("## Done Translations = {:?}", translations);
        let data = Data{wares, translations};
        let calced = data.calc_required_fabric_counts(vec![("ARG S All-round Engine Mk1".to_string(), CountsInput::WaresPerMinute(36f64))], Vec::new(), Vec::new()).unwrap();
        println!("## Calced = {:?}", calced);
    }
}
