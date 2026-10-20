//! Explicit dev experiment: a private PipeWire graph with synthetic input and no hardware monitors.
#[cfg(target_os = "linux")]
mod linux {
    use std::{
        collections::BTreeMap,
        fs::{self, File},
        io::{Read, Write},
        os::unix::fs::DirBuilderExt,
        path::{Path, PathBuf},
        process::{Child, Command, Stdio},
        thread,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };
    const RATE: usize = 48_000;
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
    struct Process(Child);
    impl Drop for Process {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    struct Directory(PathBuf);
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn wait(process: &mut Process, seconds: u64) -> Result<()> {
        let end = Instant::now() + Duration::from_secs(seconds);
        loop {
            if let Some(status) = process.0.try_wait()? {
                return if status.success() {
                    Ok(())
                } else {
                    Err("experiment process failed".into())
                };
            }
            if Instant::now() >= end {
                return Err("experiment process timed out".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
    fn run(mut command: Command, path: &Path, seconds: u64) -> Result<String> {
        command.stdout(File::create(path)?);
        let mut process = Process(command.spawn()?);
        wait(&mut process, seconds)?;
        metadata(path)
    }
    fn metadata(path: &Path) -> Result<String> {
        let mut text = String::new();
        File::open(path)?.take(512 * 1024 + 1).read_to_string(&mut text)?;
        if text.len() > 512 * 1024 {
            return Err("experiment metadata exceeds its bound".into());
        }
        Ok(text)
    }
    fn command(program: &str, args: &[&str], env: &BTreeMap<&str, String>) -> Command {
        let mut cmd = Command::new(program);
        cmd.args(args).envs(env);
        for key in [
            "PIPEWIRE_CONFIG_PREFIX",
            "PIPEWIRE_CONFIG_NAME",
            "PULSE_SINK",
            "PULSE_SOURCE",
        ] {
            cmd.env_remove(key);
        }
        cmd
    }
    pub fn main() -> Result<()> {
        if std::env::args().nth(1).as_deref() == Some("--isolated-child") {
            return measure();
        }
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let root =
            Directory(std::env::temp_dir().join(format!("vults-echo-spike-{}-{stamp}", std::process::id())));
        fs::DirBuilder::new().mode(0o700).create(&root.0)?;
        let config = root.0.join("pipewire.conf");
        fs::write(&config, include_str!("pipewire.conf"))?;
        let client = root.0.join("config/pipewire");
        fs::create_dir_all(client.join("client.conf.d"))?;
        fs::write(client.join("client.conf"), include_str!("client.conf"))?;
        let wp = root.0.join("config/wireplumber/wireplumber.conf.d");
        fs::create_dir_all(&wp)?;
        fs::write(wp.join("99-vults-spike.conf"), include_str!("wireplumber.conf"))?;
        fs::copy(
            "/usr/share/wireplumber/wireplumber.conf",
            root.0.join("config/wireplumber/wireplumber.conf"),
        )?;
        fs::write(root.0.join("pulse-client.conf"), "autospawn = no\n")?;
        fs::write(root.0.join("cookie"), [0_u8; 256])?;
        let path = |suffix: &str| root.0.join(suffix).to_string_lossy().into_owned();
        let env = BTreeMap::from([
            ("XDG_RUNTIME_DIR", path("")),
            ("PIPEWIRE_RUNTIME_DIR", path("")),
            ("PIPEWIRE_CONFIG_DIR", path("config/pipewire")),
            ("PIPEWIRE_STATE_DIR", path("state")),
            ("WIREPLUMBER_CONFIG_DIR", path("config/wireplumber")),
            ("WIREPLUMBER_DATA_DIR", "/usr/share/wireplumber".into()),
            ("PULSE_CLIENTCONFIG", path("pulse-client.conf")),
            ("XDG_CONFIG_HOME", path("config")),
            ("XDG_CONFIG_DIRS", path("config")),
            ("XDG_CACHE_HOME", path("cache")),
            ("XDG_STATE_HOME", path("state")),
            ("PIPEWIRE_REMOTE", "vults-echo-spike".into()),
            ("PULSE_SERVER", format!("unix:{}", path("pulse/native"))),
            ("PULSE_RUNTIME_PATH", path("pulse")),
            ("PULSE_COOKIE", path("cookie")),
            (
                "DBUS_SESSION_BUS_ADDRESS",
                format!("unix:path={}", path("no-bus")),
            ),
            (
                "DBUS_SYSTEM_BUS_ADDRESS",
                format!("unix:path={}", path("no-system-bus")),
            ),
            ("VULTS_ECHO_SPIKE_DIR", path("")),
        ]);
        let mut core = command("pipewire", &[], &env);
        core.arg("-c").arg(&config);
        let log = File::create(root.0.join("core.log"))?;
        core.stdout(log.try_clone()?).stderr(log);
        let mut core = Process(core.spawn()?);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !root.0.join("pulse/native").exists() {
            if core.0.try_wait()?.is_some() || Instant::now() >= deadline {
                return Err("private PipeWire failed to start".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
        let log = File::create(root.0.join("policy.log"))?;
        let mut policy = command("wireplumber", &["--profile=policy"], &env);
        policy.stdout(log.try_clone()?).stderr(log);
        let _policy = Process(policy.spawn()?);
        for args in [
            vec![
                "load-module",
                "module-null-sink",
                "sink_name=spike_mic",
                "channels=1",
                "rate=48000",
            ],
            vec![
                "load-module",
                "module-null-sink",
                "sink_name=spike_speaker",
                "channels=1",
                "rate=48000",
            ],
            vec![
                "load-module",
                "module-echo-cancel",
                "source_name=spike_clean",
                "sink_name=spike_reference",
                "source_master=spike_mic.monitor",
                "sink_master=spike_speaker",
                "aec_method=webrtc",
                "rate=48000",
                "channels=1",
                "aec_args=\"analog_gain_control=0 digital_gain_control=0 noise_suppression=0 high_pass_filter=0\"",
            ],
        ] {
            run(command("pactl", &args, &env), &root.0.join("module.log"), 5)?;
        }
        let graph = run(command("pw-dump", &[], &env), &root.0.join("graph.json"), 5)?;
        if graph.contains("api.alsa") || graph.contains("api.bluez") {
            return Err("hardware nodes are forbidden in this experiment".into());
        }
        let mut child = command(
            std::env::current_exe()?
                .to_str()
                .ok_or("invalid executable path")?,
            &["--isolated-child"],
            &env,
        );
        child.stderr(File::create(root.0.join("measure.log"))?);
        let result = run(child, &root.0.join("result.json"), 30);
        match result {
            Ok(text) => print!("{text}"),
            Err(error) => {
                eprintln!("{}", fs::read_to_string(root.0.join("measure.log"))?);
                eprintln!("{}", fs::read_to_string(root.0.join("policy.log"))?);
                return Err(error);
            }
        }
        Ok(())
    }
    fn signal() -> (Vec<f32>, Vec<f32>) {
        let mut seed = 0x4ec3_u32;
        let reference: Vec<_> = (0..RATE * 8)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                ((seed >> 8) as f32 / 16777216.0 - 0.5) * 0.3
            })
            .collect();
        let microphone = (0..reference.len())
            .map(|i| {
                let echo = if i >= 960 { reference[i - 960] * 0.5 } else { 0.0 };
                let t = i as f64 / RATE as f64;
                let near = if i >= RATE * 6 {
                    0.03 * ((t * 300.0 * std::f64::consts::TAU).sin()
                        + (t * 700.0 * std::f64::consts::TAU).sin()) as f32
                } else {
                    0.0
                };
                echo + near
            })
            .collect();
        (reference, microphone)
    }
    fn rms(samples: &[f32]) -> f64 {
        (samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / samples.len() as f64).sqrt()
    }
    fn amplitude(samples: &[f32], frequency: f64) -> f64 {
        let (mut real, mut imaginary) = (0.0, 0.0);
        for (i, sample) in samples.iter().enumerate() {
            let phase = i as f64 * frequency * std::f64::consts::TAU / RATE as f64;
            real += f64::from(*sample) * phase.cos();
            imaginary += f64::from(*sample) * phase.sin();
        }
        2.0 * real.hypot(imaginary) / samples.len() as f64
    }
    fn player(target: &str) -> Result<Process> {
        let child = Command::new("pw-cat")
            .args([
                "--playback",
                "--raw",
                "--rate=48000",
                "--channels=1",
                "--format=f32",
                "--latency=10ms",
                "--target",
                target,
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Process(child))
    }
    fn measure() -> Result<()> {
        let root = PathBuf::from(std::env::var_os("VULTS_ECHO_SPIKE_DIR").ok_or("private runtime required")?);
        if std::env::var("PULSE_SERVER")? != format!("unix:{}", root.join("pulse/native").display())
            || std::env::var("PIPEWIRE_REMOTE")? != "vults-echo-spike"
        {
            return Err("private endpoints required".into());
        }
        let pairs = vults_audio::echo::discover().map_err(|_| "echo discovery failed")?;
        if pairs.len() != 1 || pairs[0].source() != "spike_clean" || pairs[0].sink() != "spike_reference" {
            return Err("expected one paired synthetic route".into());
        }
        if !vults_audio::echo::present(&pairs[0]).map_err(|_| "echo recheck failed")? {
            return Err("echo route changed".into());
        }
        let mut recorder = Process(
            Command::new("pw-cat")
                .args([
                    "--record",
                    "--raw",
                    "--rate=48000",
                    "--channels=1",
                    "--format=f32",
                    "--latency=10ms",
                    "--target",
                    "spike_clean",
                    "-",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()?,
        );
        let stdout = recorder.0.stdout.take().ok_or("missing capture pipe")?;
        let reader = thread::spawn(move || {
            let mut bytes = Vec::new();
            let result = stdout.take((RATE * 12 * 4 + 1) as u64).read_to_end(&mut bytes);
            result.map(|_| bytes)
        });
        let (reference, microphone) = signal();
        let input_rms = rms(&reference) * 0.5;
        let mut players = Vec::new();
        let mut writers = Vec::new();
        for (target, samples) in [("spike_reference", reference), ("spike_mic", microphone)] {
            let mut process = player(target)?;
            let mut input = process.0.stdin.take().ok_or("missing playback pipe")?;
            writers.push(thread::spawn(move || {
                for block in samples.chunks(480) {
                    let bytes: Vec<_> = block.iter().flat_map(|sample| sample.to_le_bytes()).collect();
                    input.write_all(&bytes)?;
                }
                Ok::<_, std::io::Error>(())
            }));
            players.push(process);
        }
        thread::sleep(Duration::from_secs(9));
        drop(players);
        drop(recorder);
        for writer in writers {
            writer.join().map_err(|_| "playback worker failed")??;
        }
        let bytes = reader.join().map_err(|_| "capture worker failed")??;
        if !bytes.len().is_multiple_of(4) || bytes.len() > RATE * 12 * 4 {
            return Err("invalid bounded capture".into());
        }
        let samples: Vec<_> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .copied()
            .map(f32::from_le_bytes)
            .collect();
        if samples.iter().any(|s| !s.is_finite()) {
            return Err("nonfinite capture".into());
        }
        let onset = samples
            .iter()
            .position(|s| s.abs() > 0.0001)
            .ok_or("no captured synthetic audio")?;
        let far = samples
            .get(onset + RATE * 3..onset + RATE * 9 / 2)
            .ok_or("short echo capture")?;
        let near = samples
            .get(onset + RATE * 63 / 10..onset + RATE * 15 / 2)
            .ok_or("short near-end capture")?;
        let erle = 20.0 * (input_rms / rms(far).max(1e-12)).log10();
        let gain300 = 20.0 * (amplitude(near, 300.0) / 0.03).max(1e-12).log10();
        let gain700 = 20.0 * (amplitude(near, 700.0) / 0.03).max(1e-12).log10();
        let passed = erle >= 15.0 && (-3.0..=3.0).contains(&gain300) && (-3.0..=3.0).contains(&gain700);
        println!(
            "{{\"rate\":48000,\"frames\":{},\"onset_frames\":{onset},\"echo_reduction_db\":{erle:.3},\"near_300_gain_db\":{gain300:.3},\"near_700_gain_db\":{gain700:.3},\"synthetic_gate_passed\":{passed},\"physical_acceptance\":false}}",
            samples.len()
        );
        Ok(())
    }
    #[test]
    fn oversized_metadata_is_rejected_instead_of_scanning_a_prefix() {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = Directory(
            std::env::temp_dir().join(format!("vults-echo-metadata-{}-{stamp}", std::process::id())),
        );
        fs::DirBuilder::new().mode(0o700).create(&root.0).unwrap();
        let file = root.0.join("graph.json");
        fs::write(&file, "x".repeat(512 * 1024)).unwrap();
        assert_eq!(metadata(&file).unwrap().len(), 512 * 1024);
        fs::write(&file, "x".repeat(512 * 1024 + 1)).unwrap();
        assert!(metadata(&file).is_err());
    }
    #[test]
    fn scoring_preserves_reference_amplitude_and_attenuation() {
        let sine: Vec<_> = (0..RATE)
            .map(|i| 0.03 * (i as f64 * 300.0 * std::f64::consts::TAU / RATE as f64).sin() as f32)
            .collect();
        assert!((amplitude(&sine, 300.0) - 0.03).abs() < 1e-6);
        let quiet: Vec<_> = sine.iter().map(|s| s * 0.1).collect();
        assert!((20.0 * (rms(&sine) / rms(&quiet)).log10() - 20.0).abs() < 1e-4);
    }
}
#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    linux::main()
}
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("This experiment requires Linux.");
    std::process::exit(1);
}
