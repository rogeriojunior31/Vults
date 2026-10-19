#[cfg(target_os = "linux")]
fn main() -> Result<(), String> {
    use std::path::PathBuf;
    use vults_audio::meeting;

    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() == 2 && args[0] == "--recover" {
        meeting::recover(&PathBuf::from(&args[1]))?;
        println!("Temporary meeting audio recovered. No audio stream was opened.");
        return Ok(());
    }
    if args.is_empty() {
        println!("{:?}", meeting::sources()?);
        println!("Inventory only. No audio stream was opened.");
        return Ok(());
    }
    let interactive = args.len() == 5 && args[4] == "--interactive";
    if (!interactive && args.len() != 4)
        || args[0] != "--record-seconds"
        || args[2] != "--consent-temporary-audio"
    {
        return Err(
            "usage: meeting-capture [--recover PRIVATE_DIRECTORY | --record-seconds 1..60 --consent-temporary-audio PRIVATE_DIRECTORY [--interactive]]"
                .into(),
        );
    }
    let seconds = args[1].parse().map_err(|_| "invalid duration")?;
    let root = PathBuf::from(&args[3]);
    println!(
        "Recording microphone and system output into private temporary WAVs; tell other people before recording. Files are deleted before exit, or on the next recording after a crash."
    );
    if !interactive {
        println!("{:?}", meeting::record_for(&root, seconds)?);
        return Ok(());
    }
    use std::io::BufRead;
    use std::sync::mpsc;
    use std::time::Duration;

    let mut recording = meeting::Recording::start(&root, seconds)?;
    println!("Type stop to finish or cancel to discard. End of input cancels.");
    let (tx, commands) = mpsc::sync_channel(4);
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            match line {
                Ok(line) if matches!(line.trim(), "stop" | "cancel") => {
                    if tx.send(line.trim() == "cancel").is_err() {
                        return;
                    }
                }
                Ok(_) => eprintln!("Type stop or cancel."),
                Err(_) => break,
            }
        }
        let _ = tx.send(true);
    });
    let mut previous = None;
    loop {
        for cancel in commands.try_iter().take(4) {
            if cancel {
                recording.cancel();
            } else {
                recording.stop();
            }
        }
        let status = recording.status();
        if previous != Some(status.phase) {
            println!("{status:?}");
            previous = Some(status.phase);
        }
        if let Some(result) = recording.take_result() {
            println!("{:?}", result?);
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("The meeting capture spike supports Linux only.");
    std::process::exit(1);
}
