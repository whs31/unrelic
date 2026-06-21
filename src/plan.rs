use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

use crate::{Result, UnrelicError};

#[derive(Debug, Clone)]
pub struct PlanOptions {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub recursive: bool,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConvertJob {
    pub input: PathBuf,
    pub output: PathBuf,
    pub temp_output: PathBuf,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SkippedJob {
    pub input: PathBuf,
    pub output: PathBuf,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SkipReason {
    OutputExists,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct ConversionPlan {
    pub jobs: Vec<ConvertJob>,
    pub skipped: Vec<SkippedJob>,
}

impl ConversionPlan {
    pub fn total_files(&self) -> usize {
        self.jobs.len() + self.skipped.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty() && self.skipped.is_empty()
    }
}

pub fn build_plan(options: &PlanOptions) -> Result<ConversionPlan> {
    if !options.input.exists() {
        return Err(UnrelicError::InputMissing {
            path: options.input.clone(),
        });
    }

    if options.input.is_file() {
        return plan_file(options);
    }

    if options.input.is_dir() {
        return plan_directory(options);
    }

    Err(UnrelicError::InputUnsupported {
        path: options.input.clone(),
    })
}

fn plan_file(options: &PlanOptions) -> Result<ConversionPlan> {
    let output = resolve_file_output(&options.input, options.output.as_deref());
    Ok(plan_candidate(
        options.input.clone(),
        output,
        options.overwrite,
    ))
}

fn plan_directory(options: &PlanOptions) -> Result<ConversionPlan> {
    if let Some(output) = &options.output {
        if output.exists() && !output.is_dir() {
            return Err(UnrelicError::OutputMustBeDirectory {
                path: output.clone(),
            });
        }
    }

    let mut inputs = Vec::new();
    let walker = if options.recursive {
        WalkDir::new(&options.input).min_depth(1)
    } else {
        WalkDir::new(&options.input).min_depth(1).max_depth(1)
    };

    for entry in walker {
        let entry = entry.map_err(|source| UnrelicError::WalkDir {
            path: options.input.clone(),
            source,
        })?;
        let path = entry.path();
        if entry.file_type().is_file() && is_mpg_path(path) {
            inputs.push(path.to_path_buf());
        }
    }

    inputs.sort();

    if inputs.is_empty() {
        return Err(UnrelicError::NoInputFiles {
            path: options.input.clone(),
        });
    }

    let mut plan = ConversionPlan::default();
    for input in inputs {
        let output = resolve_directory_output(&options.input, &input, options.output.as_deref());
        let candidate = plan_candidate(input, output, options.overwrite);
        plan.jobs.extend(candidate.jobs);
        plan.skipped.extend(candidate.skipped);
    }

    Ok(plan)
}

fn plan_candidate(input: PathBuf, output: PathBuf, overwrite: bool) -> ConversionPlan {
    if output.exists() && !overwrite {
        return ConversionPlan {
            jobs: Vec::new(),
            skipped: vec![SkippedJob {
                input,
                output,
                reason: SkipReason::OutputExists,
            }],
        };
    }

    ConversionPlan {
        jobs: vec![ConvertJob {
            input,
            temp_output: temp_output_for(&output),
            output,
            overwrite,
        }],
        skipped: Vec::new(),
    }
}

fn resolve_file_output(input: &Path, output: Option<&Path>) -> PathBuf {
    match output {
        Some(path) if path.exists() && path.is_dir() => path
            .join(
                input
                    .file_name()
                    .unwrap_or_else(|| OsStr::new("output.mpg")),
            )
            .with_extension("mp4"),
        Some(path) => path.to_path_buf(),
        None => input.with_extension("mp4"),
    }
}

fn resolve_directory_output(root: &Path, input: &Path, output_root: Option<&Path>) -> PathBuf {
    match output_root {
        Some(output_root) => {
            let relative = input.strip_prefix(root).unwrap_or(input);
            output_root.join(relative).with_extension("mp4")
        }
        None => input.with_extension("mp4"),
    }
}

fn temp_output_for(output: &Path) -> PathBuf {
    let file_name = output
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("output.mp4");
    output.with_file_name(format!("{file_name}.unrelic-part.mp4"))
}

fn is_mpg_path(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("mpg") || extension.eq_ignore_ascii_case("mpeg")
        })
}

pub fn ensure_output_parent(job: &ConvertJob) -> Result<()> {
    if let Some(parent) = job.output.parent() {
        fs::create_dir_all(parent).map_err(|source| UnrelicError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_file(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, b"test").unwrap();
    }

    #[test]
    fn file_defaults_to_sibling_mp4() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("clip.mpg");
        write_file(&input);

        let plan = build_plan(&PlanOptions {
            input: input.clone(),
            output: None,
            recursive: true,
            overwrite: false,
        })
        .unwrap();

        assert_eq!(plan.jobs.len(), 1);
        assert_eq!(plan.jobs[0].output, temp.path().join("clip.mp4"));
        assert!(
            plan.jobs[0]
                .temp_output
                .ends_with("clip.mp4.unrelic-part.mp4")
        );
    }

    #[test]
    fn file_output_existing_directory_uses_input_name() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("clip.mpeg");
        let output_dir = temp.path().join("converted");
        write_file(&input);
        fs::create_dir(&output_dir).unwrap();

        let plan = build_plan(&PlanOptions {
            input,
            output: Some(output_dir.clone()),
            recursive: true,
            overwrite: false,
        })
        .unwrap();

        assert_eq!(plan.jobs[0].output, output_dir.join("clip.mp4"));
    }

    #[test]
    fn directory_output_preserves_relative_paths() {
        let temp = tempfile::tempdir().unwrap();
        let input_dir = temp.path().join("input");
        let output_dir = temp.path().join("out");
        write_file(&input_dir.join("nested").join("CLIP.MPEG"));

        let plan = build_plan(&PlanOptions {
            input: input_dir,
            output: Some(output_dir.clone()),
            recursive: true,
            overwrite: false,
        })
        .unwrap();

        assert_eq!(plan.jobs.len(), 1);
        assert_eq!(
            plan.jobs[0].output,
            output_dir.join("nested").join("CLIP.mp4")
        );
    }

    #[test]
    fn directory_discovery_can_be_non_recursive() {
        let temp = tempfile::tempdir().unwrap();
        let input_dir = temp.path().join("input");
        write_file(&input_dir.join("top.mpg"));
        write_file(&input_dir.join("nested").join("child.mpg"));

        let plan = build_plan(&PlanOptions {
            input: input_dir,
            output: None,
            recursive: false,
            overwrite: false,
        })
        .unwrap();

        assert_eq!(plan.jobs.len(), 1);
        assert!(plan.jobs[0].input.ends_with("top.mpg"));
    }

    #[test]
    fn directory_discovery_ignores_non_mpg_files() {
        let temp = tempfile::tempdir().unwrap();
        let input_dir = temp.path().join("input");
        write_file(&input_dir.join("top.mpg"));
        write_file(&input_dir.join("notes.txt"));
        write_file(&input_dir.join("already.mp4"));

        let plan = build_plan(&PlanOptions {
            input: input_dir,
            output: None,
            recursive: true,
            overwrite: false,
        })
        .unwrap();

        assert_eq!(plan.jobs.len(), 1);
        assert!(plan.jobs[0].input.ends_with("top.mpg"));
    }

    #[test]
    fn existing_output_is_skipped_without_overwrite() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("clip.mpg");
        let output = temp.path().join("clip.mp4");
        write_file(&input);
        write_file(&output);

        let plan = build_plan(&PlanOptions {
            input,
            output: None,
            recursive: true,
            overwrite: false,
        })
        .unwrap();

        assert!(plan.jobs.is_empty());
        assert_eq!(plan.skipped.len(), 1);
        assert_eq!(plan.skipped[0].output, output);
    }

    #[test]
    fn existing_output_is_job_with_overwrite() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("clip.mpg");
        let output = temp.path().join("clip.mp4");
        write_file(&input);
        write_file(&output);

        let plan = build_plan(&PlanOptions {
            input,
            output: None,
            recursive: true,
            overwrite: true,
        })
        .unwrap();

        assert_eq!(plan.jobs.len(), 1);
        assert!(plan.skipped.is_empty());
    }

    #[test]
    fn directory_output_existing_file_is_invalid() {
        let temp = tempfile::tempdir().unwrap();
        let input_dir = temp.path().join("input");
        let output_file = temp.path().join("out.mp4");
        write_file(&input_dir.join("top.mpg"));
        write_file(&output_file);

        let error = build_plan(&PlanOptions {
            input: input_dir,
            output: Some(output_file),
            recursive: true,
            overwrite: false,
        })
        .unwrap_err();

        assert!(matches!(error, UnrelicError::OutputMustBeDirectory { .. }));
    }
}
