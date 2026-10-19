//! Explicit local fixture runner. Reads a WAV; never records, downloads or saves audio.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::AtomicBool;

fn main() -> Result<(), String> {
    // SAFETY: synchronous entry, before any worker or native runtime is created.
    unsafe { std::env::set_var("ORT_DISABLE_TELEMETRY", "1") };
    let mut args: Vec<_> = std::env::args().skip(1).collect();
    let assets = if args.len() >= 2 && args[args.len() - 2] == "--speaker-assets" {
        let path = args.pop();
        args.pop();
        path
    } else {
        None
    };
    if !matches!(args.len(), 4 | 6)
        || args[0] != "--model"
        || args[2] != "--wav"
        || (args.len() == 6 && args[4] != "--language")
    {
        return Err(
            "usage: meeting-transcribe --model INSTALLED_MODEL --wav CONSENTED_WAV [--language CODE] [--speaker-assets INSTALLED_DIRECTORY]".into(),
        );
    }
    let input = BufReader::new(File::open(&args[3]).map_err(|e| e.to_string())?);
    let transcriber = vults_voice::Transcriber::load(Path::new(&args[1]), false)?;
    let stop = AtomicBool::new(false);
    if let Some(assets) = assets {
        #[cfg(all(feature = "diarization", feature = "meeting-assets"))]
        {
            let bundle = vults_voice::meeting::assets::installed(Path::new(&assets), &Default::default())?;
            let result = vults_voice::meeting::diarization::transcribe_wav(
                &transcriber,
                bundle.as_ref(),
                input,
                args.get(5).map(String::as_str),
                &stop,
                |_| {},
                |_| {},
            )?
            .ok_or("transcription cancelled")?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "experimental": true, "productionQualified": false,
                    "attribution": format!("{:?}", result.attribution),
                    "durationMs": result.transcript.duration_ms,
                    "segments": result.transcript.segments.iter().map(|s| serde_json::json!({
                        "startMs": s.segment.start_ms, "endMs": s.segment.end_ms,
                        "text": s.segment.text, "speaker": s.speaker,
                    })).collect::<Vec<_>>(),
                }))
                .map_err(|e| e.to_string())?
            );
            return Ok(());
        }
        #[cfg(not(all(feature = "diarization", feature = "meeting-assets")))]
        {
            let _ = assets;
            return Err("speaker comparison requires the diarization and meeting-assets features".into());
        }
    }
    let transcript = vults_voice::meeting::transcribe_wav(
        &transcriber,
        input,
        args.get(5).map(String::as_str),
        &stop,
        |progress| {
            eprintln!(
                "{} / {} ms; {} segments",
                progress.completed_ms, progress.total_ms, progress.segments
            )
        },
    )?
    .ok_or("transcription cancelled")?;
    let output = serde_json::json!({
        "durationMs": transcript.duration_ms,
        "segments": transcript.segments.iter().map(|s| serde_json::json!({
            "startMs": s.start_ms, "endMs": s.end_ms, "text": s.text,
        })).collect::<Vec<_>>(),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?
    );
    Ok(())
}
