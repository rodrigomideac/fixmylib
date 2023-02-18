use std::fmt;
use std::fmt::Formatter;

pub trait ContentReplacer {
    fn replace_tokens(&self, str: &str) -> String {
        let mut replaced = str.to_string();
        for (token, value) in self.token_pairs() {
            replaced = replaced.replace(&token, &value);
        }
        replaced
    }
    fn tokens(&self) -> Vec<String> {
        self.token_pairs()
            .into_iter()
            .map(|(token, _)| token)
            .collect()
    }
    fn main_identifier(&self) -> String;
    fn token_pairs(&self) -> Vec<(String, String)>;
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct FileProperties {
    pub filestem: String,
    pub full_path: String,
    pub extension: String,
    pub path: String,
    pub folder_path: String,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct FolderProperties {
    pub full_path: String,
    pub path: String,
}

impl ContentReplacer for FolderProperties {
    fn main_identifier(&self) -> String {
        self.path.clone()
    }

    fn token_pairs(&self) -> Vec<(String, String)> {
        vec![
            (
                "<input-folder-full-path>".to_string(),
                self.full_path.to_string(),
            ),
            ("<input-folder-path>".to_string(), self.path.to_string()),
        ]
    }
}

impl fmt::Display for FileProperties {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.full_path)
    }
}

impl ContentReplacer for FileProperties {
    fn main_identifier(&self) -> String {
        self.path.clone()
    }

    fn token_pairs(&self) -> Vec<(String, String)> {
        vec![
            (
                "<input-file-full-path>".to_string(),
                self.full_path.to_string(),
            ),
            ("<file-stem>".to_string(), self.filestem.to_string()),
            ("<file-path>".to_string(), self.path.to_string()),
            ("<folder-path>".to_string(), self.folder_path.to_string()),
            ("<file-extension>".to_string(), self.extension.to_string()),
        ]
    }
}
