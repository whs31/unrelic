use std::{process::ExitCode, time::Duration};

use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use unrelic::{
    Result,
    cli::Cli,
    ffmpeg::{EncoderSettings, convert_job, probe_duration, probe_frame_rate, probe_source_scan},
    plan::{PlanOptions, SkipReason, build_plan},
    tools::resolve_tools,
};

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<u8> {
    let plan = build_plan(&PlanOptions {
        input: cli.input.clone(),
        output: cli.output.clone(),
        recursive: cli.recursive,
        overwrite: cli.overwrite,
    })?;

    if plan.is_empty() {
        eprintln!("No files to convert.");
        return Ok(0);
    }

    let total = plan.total_files();
    let mut converted = 0usize;
    let mut skipped = 0usize;
    let mut failed = Vec::new();

    let multi = MultiProgress::new();
    let overall = multi.add(ProgressBar::new(total as u64));
    overall.set_style(progress_style(
        "{spinner:.green} files [{bar:40.cyan/blue}] {pos}/{len} {msg}",
        "##-",
    ));

    for skipped_job in &plan.skipped {
        skipped += 1;
        let reason = match skipped_job.reason {
            SkipReason::OutputExists => "output exists",
        };
        overall.println(format!(
            "skip: {} -> {} ({reason})",
            skipped_job.input.display(),
            skipped_job.output.display()
        ));
        overall.inc(1);
    }

    if plan.jobs.is_empty() {
        overall.finish_and_clear();
        eprintln!("Summary: {converted} converted, {skipped} skipped, 0 failed.");
        return Ok(0);
    }

    let tools = resolve_tools(cli.ffmpeg.as_deref(), cli.ffprobe.as_deref())?;
    let settings = EncoderSettings {
        crf: cli.crf,
        preset: cli.preset,
        audio_bitrate: cli.audio_bitrate,
    };

    for job in &plan.jobs {
        overall.set_message(job.input.display().to_string());
        let file_bar = multi.add(ProgressBar::new_spinner());
        file_bar.set_message(job.input.display().to_string());

        let duration = match probe_duration(&tools.ffprobe, &job.input) {
            Ok(duration) => duration,
            Err(error) => {
                file_bar.finish_and_clear();
                failed.push(format!("{}: {error}", job.input.display()));
                overall.println(format!("fail: {} ({error})", job.input.display()));
                overall.inc(1);
                continue;
            }
        };
        let source_scan = match probe_source_scan(&tools.ffprobe, &job.input) {
            Ok(source_scan) => source_scan,
            Err(error) => {
                file_bar.finish_and_clear();
                failed.push(format!("{}: {error}", job.input.display()));
                overall.println(format!("fail: {} ({error})", job.input.display()));
                overall.inc(1);
                continue;
            }
        };
        let deinterlace = cli.deinterlace.should_deinterlace(source_scan);
        let frame_rate = match probe_frame_rate(&tools.ffprobe, &job.input) {
            Ok(frame_rate) => frame_rate,
            Err(error) => {
                file_bar.finish_and_clear();
                failed.push(format!("{}: {error}", job.input.display()));
                overall.println(format!("fail: {} ({error})", job.input.display()));
                overall.inc(1);
                continue;
            }
        };

        configure_file_progress(&file_bar, duration);

        match convert_job(
            job,
            &tools,
            &settings,
            duration,
            deinterlace,
            frame_rate.as_ref(),
            &file_bar,
        ) {
            Ok(()) => {
                converted += 1;
                file_bar.finish_and_clear();
                overall.println(format!(
                    "done: {} -> {}",
                    job.input.display(),
                    job.output.display()
                ));
            }
            Err(error) => {
                file_bar.finish_and_clear();
                failed.push(format!("{}: {error}", job.input.display()));
                overall.println(format!("fail: {} ({error})", job.input.display()));
            }
        }

        overall.inc(1);
    }

    overall.finish_and_clear();
    eprintln!(
        "Summary: {converted} converted, {skipped} skipped, {} failed.",
        failed.len()
    );
    for failure in &failed {
        eprintln!("failed: {failure}");
    }

    Ok(if failed.is_empty() { 0 } else { 1 })
}

fn configure_file_progress(progress: &ProgressBar, duration: Option<Duration>) {
    match duration {
        Some(duration) => {
            progress.set_style(progress_style(
                "{spinner:.green} {msg} [{bar:40.cyan/blue}] {pos}/{len}ms {percent}%",
                "##-",
            ));
            progress.set_length(duration_millis(duration));
            progress.set_position(0);
        }
        None => {
            progress.set_style(
                ProgressStyle::with_template("{spinner:.green} {msg} {elapsed_precise}")
                    .unwrap_or_else(|_| ProgressStyle::default_spinner()),
            );
            progress.enable_steady_tick(Duration::from_millis(100));
        }
    }
}

fn progress_style(template: &str, chars: &str) -> ProgressStyle {
    ProgressStyle::with_template(template)
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .progress_chars(chars)
}

fn duration_millis(duration: Duration) -> u64 {
    duration.as_millis().try_into().unwrap_or(u64::MAX).max(1)
}
