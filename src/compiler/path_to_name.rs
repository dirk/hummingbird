use std::path::PathBuf;

use regex::Regex;

pub fn path_to_name<P: Into<PathBuf>>(path: P) -> String {
    let slashes = Regex::new(r"[/\\]").unwrap();
    let repeated_underscores = Regex::new(r"_+").unwrap();
    let extension = Regex::new(r"\.hb$").unwrap();
    let invalid = Regex::new(r"[^A-Za-z0-9_]").unwrap();
    let simplify = Regex::new(r"^_?(?P<inner>.+)_?$").unwrap();

    let path = path.into().to_str().unwrap().to_string();
    let without_slashes = slashes.replace_all(&path, "_");
    let condensed_underscores = repeated_underscores.replace_all(&without_slashes, "_");
    let without_extension = extension.replace(&condensed_underscores, "");
    let without_invalid = invalid.replace_all(&without_extension, "");
    let simplified = simplify.replace(&without_invalid, "$inner");

    simplified.to_string()
}
