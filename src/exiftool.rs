use std::path::Path;

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Exiftool {
    pub file_type: String,
    #[serde(rename = "MIMEType")]
    pub mime_type: String,
}

#[derive(Debug)]
pub enum ExiftoolError {
    Io(std::io::Error),
    Status(std::process::Output),
    Deserialize(serde_json::Error),
    MissingElement,
}

pub fn exiftool_on_file(path: impl AsRef<Path>) -> Result<Exiftool, ExiftoolError> {
    exiftool_on_dir(path).and_then(|e_list| {
        if let Some(e) = e_list.first() {
            Ok(e.clone())
        } else {
            Err(ExiftoolError::MissingElement)
        }
    })
}

pub fn exiftool_on_dir(path: impl AsRef<Path>) -> Result<Vec<Exiftool>, ExiftoolError> {
    let path = path.as_ref();

    let mut cmd = std::process::Command::new("exiftool");

    cmd.args(["-j"]);
    cmd.arg(path);

    let out = cmd.output().map_err(ExiftoolError::Io)?;
    if !out.status.success() {
        return Err(ExiftoolError::Status(out));
    }
    let _exiftool_string = String::from_utf8(out.stdout.clone()).unwrap();
    let exiftool_list: Vec<Exiftool> = serde_json::from_slice(&out.stdout)
        .map_err(ExiftoolError::Deserialize)
        .unwrap();
    Ok(exiftool_list)
}
