# unrelic

Tiny MPG/MPEG to MP4 converter with H.264 video and AAC audio.

`unrelic` is a Rust CLI that drives FFmpeg for the actual transcoding work. It
looks for `ffmpeg` and `ffprobe` next to the `unrelic` binary first, then falls
back to `PATH`. Release artifacts for Windows bundle FFmpeg binaries.

## Why?
Because my girlfriend received a rather old videocamera which outputs videos in the format unsuitable for the Windows Media Player and I didn't want to bother her with installing raw ffmpeg.

## Usage

```sh
unrelic <INPUT> [OPTIONS]
```

Examples:

```sh
unrelic movie.mpg
unrelic ./old-videos --output ./converted
unrelic movie.mpeg --output movie-fixed.mp4 --overwrite
unrelic ./old-videos --overwrite --deinterlace always
```

Options:

```text
Arguments:
  <INPUT>  MPG/MPEG file or directory to convert

Options:
  -o, --output <PATH>              Output file or directory
      --overwrite                  Replace existing MP4 outputs
      --no-recursive               Only scan the top level of a directory
      --ffmpeg <PATH>              Path to ffmpeg
      --ffprobe <PATH>             Path to ffprobe
      --crf <CRF>                  H.264 CRF quality, 1-51 [default: 23]
      --preset <PRESET>            x264 preset [default: medium]
      --audio-bitrate <BITRATE>    AAC audio bitrate [default: 192k]
      --deinterlace <MODE>         Deinterlace video: auto, always, never [default: auto]
  -h, --help                       Print help
  -V, --version                    Print version
```

By default, `unrelic` probes the source video field order and applies FFmpeg's
`bwdif` deinterlacer for interlaced MPG/MPEG input. This avoids the horizontal
combing artifacts that can show up when old camcorder footage is encoded as
progressive H.264. If a file still shows stripes, reconvert it with
`--overwrite --deinterlace always`.

## Windows releases

Windows releases include:

- `unrelic-windows-x86_64.msi`: a per-user installer that installs `unrelic`,
  bundled FFmpeg tools, and license files into
  `%LocalAppData%\Programs\unrelic`, then adds that directory to the user PATH
  and adds Explorer context-menu actions for `.mpg`, `.mpeg`, and folders.
- `unrelic-windows-x86_64.zip`: a portable archive with the same binaries,
  context-menu helper script, and license files.

Both Windows artifacts include FFmpeg binaries from BtbN's GPL FFmpeg builds, so
the artifacts also include FFmpeg license/readme files.
