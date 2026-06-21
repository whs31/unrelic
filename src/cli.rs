use std::{fmt, path::PathBuf};

use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Parser)]
#[command(
    name = "unrelic",
    version,
    about = "Convert legacy MPG/MPEG video to MP4 with H.264 video and AAC audio"
)]
pub struct Cli {
    #[arg(value_name = "INPUT", help = "MPG/MPEG file or directory to convert")]
    pub input: PathBuf,

    #[arg(short, long, value_name = "PATH", help = "Output file or directory")]
    pub output: Option<PathBuf>,

    #[arg(long, help = "Replace existing MP4 outputs")]
    pub overwrite: bool,

    #[arg(
        long = "no-recursive",
        action = clap::ArgAction::SetFalse,
        help = "Only scan the top level of a directory"
    )]
    pub recursive: bool,

    #[arg(long, value_name = "PATH", help = "Path to ffmpeg")]
    pub ffmpeg: Option<PathBuf>,

    #[arg(long, value_name = "PATH", help = "Path to ffprobe")]
    pub ffprobe: Option<PathBuf>,

    #[arg(
        long,
        default_value_t = 23,
        value_parser = clap::value_parser!(u8).range(1..=51),
        help = "H.264 CRF quality, from 1 (largest/best) to 51 (smallest/worst)"
    )]
    pub crf: u8,

    #[arg(long, default_value_t = Preset::Medium, value_enum, help = "x264 encoding preset")]
    pub preset: Preset,

    #[arg(
        long,
        default_value = "192k",
        value_name = "BITRATE",
        help = "AAC audio bitrate"
    )]
    pub audio_bitrate: String,

    #[arg(
        long,
        default_value_t = DeinterlaceMode::Auto,
        value_enum,
        help = "Deinterlace video: auto probes MPG field order, always forces bwdif, never disables it"
    )]
    pub deinterlace: DeinterlaceMode,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum Preset {
    Ultrafast,
    Superfast,
    Veryfast,
    Faster,
    Fast,
    Medium,
    Slow,
    Slower,
    Veryslow,
}

impl Preset {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ultrafast => "ultrafast",
            Self::Superfast => "superfast",
            Self::Veryfast => "veryfast",
            Self::Faster => "faster",
            Self::Fast => "fast",
            Self::Medium => "medium",
            Self::Slow => "slow",
            Self::Slower => "slower",
            Self::Veryslow => "veryslow",
        }
    }
}

impl fmt::Display for Preset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum DeinterlaceMode {
    Auto,
    Always,
    Never,
}

impl fmt::Display for DeinterlaceMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Never => "never",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffmpeg::SourceScan;

    #[test]
    fn parses_defaults() {
        let cli = Cli::parse_from(["unrelic", "movie.mpg"]);

        assert_eq!(cli.input, PathBuf::from("movie.mpg"));
        assert!(cli.output.is_none());
        assert!(!cli.overwrite);
        assert!(cli.recursive);
        assert_eq!(cli.crf, 23);
        assert_eq!(cli.preset, Preset::Medium);
        assert_eq!(cli.audio_bitrate, "192k");
        assert_eq!(cli.deinterlace, DeinterlaceMode::Auto);
    }

    #[test]
    fn rejects_missing_input_argument() {
        assert!(Cli::try_parse_from(["unrelic"]).is_err());
    }

    #[test]
    fn rejects_out_of_range_crf() {
        assert!(Cli::try_parse_from(["unrelic", "movie.mpg", "--crf", "0"]).is_err());
        assert!(Cli::try_parse_from(["unrelic", "movie.mpg", "--crf", "52"]).is_err());
    }

    #[test]
    fn parses_no_recursive_flag() {
        let cli = Cli::parse_from(["unrelic", "movies", "--no-recursive"]);

        assert!(!cli.recursive);
    }

    #[test]
    fn parses_deinterlace_mode() {
        let cli = Cli::parse_from(["unrelic", "movie.mpg", "--deinterlace", "always"]);

        assert_eq!(cli.deinterlace, DeinterlaceMode::Always);
    }

    #[test]
    fn rejects_invalid_deinterlace_mode() {
        assert!(
            Cli::try_parse_from(["unrelic", "movie.mpg", "--deinterlace", "sometimes"]).is_err()
        );
    }

    #[test]
    fn deinterlace_mode_resolves_from_source_scan() {
        assert!(DeinterlaceMode::Auto.should_deinterlace(SourceScan::Interlaced));
        assert!(!DeinterlaceMode::Auto.should_deinterlace(SourceScan::Progressive));
        assert!(!DeinterlaceMode::Auto.should_deinterlace(SourceScan::Unknown));
        assert!(DeinterlaceMode::Always.should_deinterlace(SourceScan::Progressive));
        assert!(!DeinterlaceMode::Never.should_deinterlace(SourceScan::Interlaced));
    }
}
