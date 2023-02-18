use crate::FixMyLibErrors;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::fs;

use crate::scans::{ContentReplacer, FileProperties, FolderProperties};
use std::process::exit;

#[derive(Debug, PartialEq, Clone)]
pub struct Config {
    pub presets: Vec<Preset>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Preset {
    pub name: String,
    pub description: String,
    pub scans: Vec<Scan>,
}

impl Preset {
    fn from(
        yaml_preset: YamlPreset,
        yaml_config_preset: &YamlConfigPreset,
        preset_path: &str,
    ) -> Preset {
        let mut scans = Vec::new();
        for scan in yaml_preset.scans {
            let mut scripts = Vec::new();
            for script in scan.scripts {
                let script_path = format!("{preset_path}/{}", script.name);
                let script_contents = fs::read_to_string(&script_path)
                    .unwrap_or_else(|_| panic!("Failure reading script {} contents.", script_path));
                let script_contents_with_args_replaced =
                    replace_args_on_string_contents(&script_contents, yaml_config_preset);
                validate_script_contents(
                    &script_contents_with_args_replaced,
                    &script_path,
                    &scan.iterate_on,
                );

                let target_extensions = script
                    .target_extensions
                    .clone()
                    .map_or_else(Vec::new, |v| {
                        v.split(',')
                            .map(String::from)
                            .map(|s| s.to_lowercase())
                            .collect()
                    })
                    .to_owned();
                let exclude_extensions = script
                    .exclude_extensions
                    .clone()
                    .map_or_else(Vec::new, |v| {
                        v.split(',')
                            .map(String::from)
                            .map(|s| s.to_lowercase())
                            .collect()
                    })
                    .to_owned();
                scripts.push(Script {
                    name: script.name,
                    working_dir: yaml_config_preset.working_dir().clone(),
                    input_folder: yaml_config_preset.input_folder().clone(),
                    contents: script_contents_with_args_replaced,
                    extension_list: ExtensionList {
                        target: target_extensions,
                        exclude: exclude_extensions,
                    },
                });
            }
            scans.push(Scan {
                threads: scan.concurrency.unwrap_or(yaml_config_preset.concurrency()),
                iterate_on: IterateOn::from(scan.iterate_on),
                scripts,
                input_folder: yaml_config_preset.input_folder().clone(),
            })
        }
        Preset {
            name: yaml_config_preset.name.clone(),
            description: yaml_preset.description,
            scans,
        }
    }
}

fn is_valid_scan_parameter(str: &str, scan_type: &YamlIterateOn) -> bool {
    let valid_parameters = match scan_type {
        YamlIterateOn::folders => FolderProperties::default().tokens(),
        YamlIterateOn::files => FileProperties::default().tokens(),
    };

    valid_parameters.contains(&str.to_string())
}

fn validate_script_contents(contents: &str, path: &str, scan_type: &YamlIterateOn) {
    let invalid_parameters = elements_between(contents, "<", ">")
        .into_iter()
        .filter(|e| !e.contains(' '))
        .map(|e| "<".to_string() + &e + ">")
        .filter(|e| !is_valid_scan_parameter(e, scan_type))
        .collect::<Vec<_>>();

    if !invalid_parameters.is_empty() {
        println!(
            "[{}] The following parameters are not valid: {}",
            path,
            invalid_parameters.join(", ")
        );
        let valid_parameters = match scan_type {
            YamlIterateOn::folders => FolderProperties::default().tokens(),
            YamlIterateOn::files => FileProperties::default().tokens(),
        };
        println!(
            "[{}] These are valid parameters:\n\t{}",
            path,
            valid_parameters.join(",\t")
        );
        exit(1);
    }
}

fn elements_between(source: &str, start: &str, end: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut remaining_str = source;
    let mut start_position = source.find(start);
    while let Some(start_index) = start_position {
        let start_index = start_index + start.len();
        let source = &remaining_str[start_index..];
        if let Some(end_index) = source.find(end) {
            result.push(source[..end_index].to_string());
            remaining_str = &source[end_index..];
            start_position = remaining_str.find(start);
        } else {
            break;
        }
    }
    result
}

fn replace_args_on_string_contents(contents: &str, preset: &YamlConfigPreset) -> String {
    let args = &preset.args;
    let mut replaced_contents = contents.to_string();
    for (name, value) in args {
        let anchor = "<&".to_string() + name + ">";
        replaced_contents = replaced_contents.replace(&anchor, value);
    }
    let elements_without_substituition = elements_between(&replaced_contents, "<&", ">");
    if !elements_without_substituition.is_empty() {
        println!(
            "[{}] Some arguments for this preset are missing. Could not find args for <&{}>, \
        please add them in your config.yaml.",
            preset.name,
            elements_without_substituition.join(">, <&")
        );
        exit(1)
    }
    replaced_contents
}

#[derive(Debug, PartialEq, Clone)]
pub struct Scan {
    pub threads: usize,
    pub iterate_on: IterateOn,
    pub scripts: Vec<Script>,
    pub input_folder: String,
}

impl Scan {
    pub fn name(&self) -> String {
        let iterate_name = match self.iterate_on {
            IterateOn::Folders => "iterate_on_folders_",
            IterateOn::Files => "iterate_on_files_",
        };
        let script_names = self
            .scripts
            .iter()
            .map(|s| s.name.to_string())
            .collect::<Vec<_>>();
        iterate_name.to_string() + script_names.join("_and_").as_ref()
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Script {
    pub name: String,
    pub working_dir: String,
    pub input_folder: String,
    pub contents: String,
    pub extension_list: ExtensionList,
}

#[derive(Debug, PartialEq, Clone)]
pub enum IterateOn {
    Folders,
    Files,
}

impl From<YamlIterateOn> for IterateOn {
    fn from(value: YamlIterateOn) -> Self {
        match value {
            YamlIterateOn::folders => Self::Folders,
            YamlIterateOn::files => Self::Files,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct ExtensionList {
    target: Vec<String>,
    exclude: Vec<String>,
}

impl ExtensionList {
    pub fn is_desired(&self, extension: &str) -> bool {
        if self.target.is_empty() && self.exclude.is_empty() {
            return true;
        }

        let ext_lowercase = extension.to_lowercase();
        if !self.exclude.is_empty() {
            !self.exclude.contains(&ext_lowercase)
        } else {
            self.target.contains(&ext_lowercase)
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct YamlConfig {
    presets: Vec<YamlConfigPreset>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct YamlConfigPreset {
    name: String,
    args: HashMap<String, String>,
}

impl YamlConfigPreset {
    fn input_folder(&self) -> String {
        self.args
            .get("input_folder")
            .unwrap_or(&String::new())
            .to_string()
    }

    fn concurrency(&self) -> usize {
        self.args
            .get("concurrency")
            .unwrap_or(&String::new())
            .to_string()
            .parse::<usize>()
            .unwrap()
    }

    fn working_dir(&self) -> String {
        self.args
            .get("working_dir")
            .unwrap_or(&String::new())
            .to_string()
    }

    fn contains_required_fields(&self) -> Result<(), FixMyLibErrors> {
        let mut required_fields = HashMap::new();
        required_fields.insert("input_folder", false);
        required_fields.insert("concurrency", false);
        required_fields.insert("working_dir", false);

        for k in self.args.keys() {
            if required_fields.contains_key(k.as_str()) {
                required_fields.insert(k, true);
            }
        }

        let fields: Vec<_> = required_fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        let missing_fields: Vec<String> = fields
            .iter()
            .filter(|(_k, v)| !(*v))
            .map(|(k, _v)| k.to_string())
            .collect();

        if missing_fields.is_empty() {
            Ok(())
        } else {
            let mut error_text = "The following required arguments were not found for preset "
                .to_string()
                + &self.name
                + ": ";
            for field in missing_fields {
                error_text += &(field.to_string() + ",");
            }
            error_text.pop();
            Err(FixMyLibErrors::InvalidConfigPreset(error_text))
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct YamlPreset {
    name: String,
    description: String,
    args: Vec<YamlPresetArg>,
    scans: Vec<YamlPresetScan>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct YamlPresetArg {
    name: String,
    description: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct YamlPresetScan {
    iterate_on: YamlIterateOn,
    scripts: Vec<YamlPresetScanScript>,
    concurrency: Option<usize>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum YamlIterateOn {
    #[allow(non_camel_case_types)]
    folders,
    #[allow(non_camel_case_types)]
    files,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct YamlPresetScanScript {
    name: String,
    target_extensions: Option<String>,
    exclude_extensions: Option<String>,
}

fn get_preset_path(builtins_path: &str, preset_name: &str) -> String {
    format!("{builtins_path}/{preset_name}")
}

fn get_preset_yaml(
    builtins_path: &str,
    preset_name: &str,
    yaml_config_preset: &YamlConfigPreset,
) -> YamlPreset {
    let preset_path = get_preset_path(builtins_path, preset_name);
    let preset_full_path = format!("{preset_path}/preset.yaml");
    let preset_str = fs::read_to_string(&preset_full_path)
        .map_err(|e| panic!("Failure reading preset from {preset_full_path}: {:?}", e))
        .unwrap();
    let preset_str_replaced = replace_args_on_string_contents(&preset_str, yaml_config_preset);
    let inner_config: YamlPreset = serde_yaml::from_str(&preset_str_replaced)
        .expect("Failure parsing config.yaml. Please check the syntax.");
    inner_config
}

fn get_config_yaml(config_path: &str) -> Result<YamlConfig, FixMyLibErrors> {
    let config_full_path = format!("{config_path}/config.yaml");
    let config_str = fs::read_to_string(&config_full_path)
        .map_err(|e| {
            panic!(
                "Failure reading config file from {config_full_path}: {:?}",
                e
            )
        })
        .unwrap();
    let config: YamlConfig = serde_yaml::from_str(&config_str)
        .expect("Failure parsing config.yaml. Please check the syntax.");
    for preset in &config.presets {
        preset.contains_required_fields()?;
    }
    Ok(config)
}

pub fn get_config(config_path: &str, builtins_path: &str) -> Config {
    let config_yaml = get_config_yaml(config_path)
        .map_err(|e| panic!("{:?}", e))
        .unwrap();
    let mut presets = Vec::new();
    for preset_target in &config_yaml.presets {
        if preset_target.name.contains("builtins/") {
            let name = preset_target.name.replace("builtins/", "");
            let preset_path = get_preset_path(builtins_path, &name);
            let preset_yaml = get_preset_yaml(builtins_path, &name, preset_target);
            let preset = Preset::from(preset_yaml, preset_target, &preset_path);
            presets.push(preset);
        }
    }
    Config { presets }
}
