use std::process::Command;

use indicatif::ProgressBar;
use unrelic::{
    cli::Preset,
    ffmpeg::{EncoderSettings, convert_job, probe_duration},
    plan::{PlanOptions, build_plan},
    tools::resolve_tools,
};

#[test]
#[ignore = "requires ffmpeg and ffprobe on PATH"]
fn converts_generated_mpg_to_h264_aac_mp4() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("sample.mpg");
    let output = temp.path().join("sample.mp4");

    let ffmpeg = which::which("ffmpeg").unwrap();
    let ffprobe = which::which("ffprobe").unwrap();

    let status = Command::new(&ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=64x64:rate=5",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:sample_rate=44100",
            "-t",
            "1",
            "-c:v",
            "mpeg2video",
            "-c:a",
            "mp2",
        ])
        .arg(&input)
        .status()
        .unwrap();
    assert!(status.success());

    let plan = build_plan(&PlanOptions {
        input: input.clone(),
        output: None,
        recursive: true,
        overwrite: false,
    })
    .unwrap();
    let tools = resolve_tools(Some(&ffmpeg), Some(&ffprobe)).unwrap();
    let settings = EncoderSettings {
        crf: 28,
        preset: Preset::Ultrafast,
        audio_bitrate: "96k".to_owned(),
    };
    let progress = ProgressBar::hidden();
    let duration = probe_duration(&tools.ffprobe, &input).unwrap();

    convert_job(&plan.jobs[0], &tools, &settings, duration, false, &progress).unwrap();

    assert!(output.is_file());
    assert_eq!(probe_codec(&ffprobe, &output, "v:0"), "h264");
    assert_eq!(probe_codec(&ffprobe, &output, "a:0"), "aac");
}

fn probe_codec(ffprobe: &std::path::Path, input: &std::path::Path, stream: &str) -> String {
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            stream,
            "-show_entries",
            "stream=codec_name",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(input)
        .output()
        .unwrap();

    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
