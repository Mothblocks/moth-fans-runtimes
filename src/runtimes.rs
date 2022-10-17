use std::path::PathBuf;

use color_eyre::eyre::{Context, ContextCompat};
use regex::Regex;
use serde::{Deserialize, Serialize};

const RUNTIME_REGEX_PATTERN: &str = r#"The following runtime has occurred (?P<count>[0-9]+).*
runtime error: (?P<exception>.+)
proc name: (?P<proc>.+?) \((?P<proc_path>.+?)\)
  source file: (?P<source_file>.+?),(?P<line>[0-9]+)"#;

#[derive(Deserialize, Serialize)]
pub struct RuntimeBatch {
    pub count: u64,
    pub exception: String,
    pub proc_path: String,
    pub source_file: String,
    pub line: u64,

    pub best_guess_filenames: Option<BestGuessFilenames>,
}

#[derive(Deserialize, Serialize)]
pub enum BestGuessFilenames {
    Definitely(PathBuf),
    Possible(Vec<PathBuf>),
}

impl RuntimeBatch {
    fn patch_special_procs(&mut self) {
        if self.proc_path == "/proc/_stack_trace" {
            self.patch_stack_trace();
        }
    }

    fn patch_stack_trace(&mut self) {
        let stack_trace_regex = Regex::new(r".+\(((?P<filename>.+?):(?P<line>[0-9]+))\)$").unwrap();

        if let Some(captures) = stack_trace_regex.captures(&self.exception) {
            let filename = PathBuf::from(captures.name("filename").unwrap().as_str());

            self.source_file = filename.file_name().unwrap().to_string_lossy().to_string();
            self.best_guess_filenames = Some(BestGuessFilenames::Definitely(filename));

            self.line = captures.name("line").unwrap().as_str().parse().unwrap();
        }
    }
}

fn group_to_runtime_batch(captures: &regex::Captures) -> color_eyre::Result<RuntimeBatch> {
    let count = captures
        .name("count")
        .context("error getting count")?
        .as_str()
        .parse::<u64>()
        .context("error parsing count")?;

    let exception = captures
        .name("exception")
        .context("error getting exception")?
        .as_str()
        .to_string();

    let proc_path = captures
        .name("proc_path")
        .context("error getting proc_path")?
        .as_str()
        .to_string();

    let source_file = captures
        .name("source_file")
        .context("error getting source_file")?
        .as_str()
        .to_string();

    let line = captures
        .name("line")
        .context("error getting line")?
        .as_str()
        .parse::<u64>()
        .context("error parsing line")?;

    let mut runtime_batch = RuntimeBatch {
        count,
        exception,
        proc_path,
        source_file,
        line,
        best_guess_filenames: None,
    };

    runtime_batch.patch_special_procs();

    Ok(runtime_batch)
}

pub fn get_runtimes_for_round(
    runtime_condensed_txt: &str,
) -> color_eyre::Result<Vec<RuntimeBatch>> {
    let runtime_regex = Regex::new(RUNTIME_REGEX_PATTERN).unwrap();

    runtime_regex
        .captures_iter(runtime_condensed_txt)
        .map(|captures| -> color_eyre::Result<RuntimeBatch> {
            match group_to_runtime_batch(&captures) {
                Ok(runtime_batch) => Ok(runtime_batch),
                result @ Err(_) => result.context(format!(
                    "error while parsing {}",
                    captures.get(0).expect("error getting group 0").as_str()
                )),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_runtimes_for_round_191838() {
        insta::assert_json_snapshot!(get_runtimes_for_round(include_str!(
            "./test_data/191838-runtime.condensed.txt"
        ))
        .unwrap());
    }
}
