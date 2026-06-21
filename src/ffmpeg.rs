use std::{
    fs,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use indicatif::ProgressBar;

use crate::{
    Result, UnrelicError,
    cli::Preset,
    plan::{ConvertJob, ensure_output_parent},
    tools::ToolPaths,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EncoderSettings {
    pub crf: u8,
    pub preset: Preset,
    pub audio_bitrate: String,
}

pub fn probe_duration(ffprobe: &Path, input: &Path) -> Result<Option<Duration>> {
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(input)
        .output()
        .map_err(|source| UnrelicError::Spawn {
            program: ffprobe.to_path_buf(),
            source,
        })?;

    if !output.status.success() {
        return Err(UnrelicError::Probe {
            input: input.to_path_buf(),
            stderr: stderr_to_string(&output.stderr, output.status),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("N/A") {
        return Ok(None);
    }

    let seconds = match trimmed.parse::<f64>() {
        Ok(seconds) if seconds.is_finite() && seconds > 0.0 => seconds,
        _ => return Ok(None),
    };

    Ok(Some(Duration::from_secs_f64(seconds)))
}

pub fn convert_job(
    job: &ConvertJob,
    tools: &ToolPaths,
    settings: &EncoderSettings,
    duration: Option<Duration>,
    progress: &ProgressBar,
) -> Result<()> {
    ensure_output_parent(job)?;
    remove_if_exists(&job.temp_output)?;

    let mut child = Command::new(&tools.ffmpeg)
        .args([
            "-hide_banner",
            "-nostdin",
            "-loglevel",
            "error",
            "-progress",
            "pipe:1",
            "-y",
        ])
        .arg("-i")
        .arg(&job.input)
        .args([
            "-map",
            "0:v:0",
            "-map",
            "0:a?",
            "-c:v",
            "libx264",
            "-preset",
            settings.preset.as_str(),
            "-crf",
        ])
        .arg(settings.crf.to_string())
        .args([
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            &settings.audio_bitrate,
            "-movflags",
            "+faststart",
        ])
        .arg(&job.temp_output)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| UnrelicError::Spawn {
            program: tools.ffmpeg.clone(),
            source,
        })?;

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| UnrelicError::PipeUnavailable {
            program: tools.ffmpeg.clone(),
            stream: "stderr",
        })?;
    let stderr_program = tools.ffmpeg.clone();
    let stderr_handle = thread::spawn(move || read_to_string(stderr, stderr_program, "stderr"));

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| UnrelicError::PipeUnavailable {
            program: tools.ffmpeg.clone(),
            stream: "stdout",
        })?;
    read_progress(stdout, &tools.ffmpeg, duration, progress)?;

    let status = child.wait().map_err(|source| UnrelicError::Wait {
        program: tools.ffmpeg.clone(),
        source,
    })?;
    let stderr = stderr_handle
        .join()
        .unwrap_or_else(|_| Ok(String::from("stderr reader panicked")))?;

    if !status.success() {
        let _ = fs::remove_file(&job.temp_output);
        return Err(UnrelicError::Convert {
            input: job.input.clone(),
            stderr: if stderr.trim().is_empty() {
                status.to_string()
            } else {
                stderr.trim().to_owned()
            },
        });
    }

    if job.output.exists() && job.overwrite {
        fs::remove_file(&job.output).map_err(|source| UnrelicError::RemoveFile {
            path: job.output.clone(),
            source,
        })?;
    } else if job.output.exists() {
        let _ = fs::remove_file(&job.temp_output);
        return Err(UnrelicError::OutputExists {
            path: job.output.clone(),
        });
    }

    fs::rename(&job.temp_output, &job.output).map_err(|source| UnrelicError::Rename {
        from: job.temp_output.clone(),
        to: job.output.clone(),
        source,
    })?;

    progress.set_position(duration_millis(duration.unwrap_or(Duration::ZERO)));
    Ok(())
}

fn read_progress<R: Read>(
    stdout: R,
    program: &Path,
    duration: Option<Duration>,
    progress: &ProgressBar,
) -> Result<()> {
    for line in BufReader::new(stdout).lines() {
        let line = line.map_err(|source| UnrelicError::ReadPipe {
            program: program.to_path_buf(),
            stream: "stdout",
            source,
        })?;

        if let Some(time) = parse_progress_time(&line) {
            if let Some(duration) = duration {
                progress.set_position(duration_millis(time).min(duration_millis(duration)));
            } else {
                progress.set_message(format!("encoded {}", format_duration(time)));
            }
        }
    }

    Ok(())
}

fn read_to_string<R: Read>(
    mut reader: R,
    program: PathBuf,
    stream: &'static str,
) -> Result<String> {
    let mut output = String::new();
    reader
        .read_to_string(&mut output)
        .map_err(|source| UnrelicError::ReadPipe {
            program,
            stream,
            source,
        })?;
    Ok(output)
}

fn remove_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(UnrelicError::RemoveFile {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn stderr_to_string(stderr: &[u8], status: std::process::ExitStatus) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        status.to_string()
    } else {
        stderr.to_owned()
    }
}

fn parse_progress_time(line: &str) -> Option<Duration> {
    if let Some(value) = line.strip_prefix("out_time_us=") {
        return value.trim().parse::<u64>().ok().map(Duration::from_micros);
    }

    if let Some(value) = line.strip_prefix("out_time_ms=") {
        return value.trim().parse::<u64>().ok().map(Duration::from_micros);
    }

    line.strip_prefix("out_time=")
        .and_then(|value| parse_ffmpeg_timestamp(value.trim()))
}

fn parse_ffmpeg_timestamp(value: &str) -> Option<Duration> {
    let mut parts = value.split(':');
    let hours = parts.next()?.parse::<u64>().ok()?;
    let minutes = parts.next()?.parse::<u64>().ok()?;
    let seconds = parts.next()?.parse::<f64>().ok()?;
    if parts.next().is_some() || !seconds.is_finite() || minutes >= 60 || seconds >= 60.0 {
        return None;
    }

    Duration::from_secs(hours * 3600 + minutes * 60).checked_add(Duration::from_secs_f64(seconds))
}

fn duration_millis(duration: Duration) -> u64 {
    duration.as_millis().try_into().unwrap_or(u64::MAX).max(1)
}

fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ffmpeg_progress_timestamp() {
        assert_eq!(
            parse_progress_time("out_time=01:02:03.500000"),
            Some(Duration::from_millis(3_723_500))
        );
    }

    #[test]
    fn parses_ffmpeg_progress_microseconds() {
        assert_eq!(
            parse_progress_time("out_time_us=2500000"),
            Some(Duration::from_millis(2_500))
        );
        assert_eq!(
            parse_progress_time("out_time_ms=1000000"),
            Some(Duration::from_secs(1))
        );
    }

    #[test]
    fn rejects_invalid_progress_timestamp() {
        assert_eq!(parse_progress_time("out_time=not-a-time"), None);
        assert_eq!(parse_progress_time("frame=12"), None);
    }
}
