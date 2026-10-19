# V5.4 offline diarization experiment

2026-10-18. **Go for bounded V5.5 engineering; no-go for shipping automatic speaker labels yet.**
The fixed baseline finds one/two speakers in controlled single/alternating speech, but cannot
reliably identify both voices during overlap. This is a development experiment, not an enabled
Meetings feature, model-download approval or real-meeting acceptance.

## Scope and reproduction

`vults-spike` has an optional `diarization` binary. Nothing in the app depends on this feature.
It performs no recording, downloads, provider calls or file writes. Supply local, consented
meeting-format WAVs, nonempty and normalized to at most 180 seconds. The original V5.4
measurements below used mono 16 kHz PCM16 inputs. Models have fixed SHA-256 pins; the
absolute native runtime path requires its own explicit SHA-256. Symlinks/devices are rejected.
The runtime is executable native code: use a trusted, independently checked ONNX Runtime build.
A fingerprint checks artifact identity; it does not establish trust in an arbitrary library.

```sh
cargo build -p vults-spike --features diarization --bin vults-diarization-spike
cargo test -p vults-spike --features diarization --bin vults-diarization-spike
cargo clippy -p vults-spike --features diarization --all-targets -- -D warnings

target/debug/vults-diarization-spike \
  --runtime "$DIARIZATION_RUNTIME" \
  --runtime-sha256 "$DIARIZATION_RUNTIME_SHA256" \
  --segmentation "$DIARIZATION_SEGMENTATION" \
  --embedding "$DIARIZATION_EMBEDDING" \
  --wav "$DIARIZATION_FIXTURE"
```

Arguments are ordered as shown. With a custom `CARGO_TARGET_DIR`, use its `debug/` executable.
JSON contains anonymous labels, source-clock turns, model/input hashes and timings; it contains
no recognized text, embeddings or personal identities. Labels are local to one invocation.
The CLI does not mix the app's separate microphone/system clocks or merge its transcript.

The baseline uses 10-second segmentation windows, a 5-second hop and disjoint central ownership
regions. Seven powerset classes retain up to two simultaneous local voices. Only non-overlapping
speech contributes to a local embedding; less than one second stays unknown. Features are 80-bin
Kaldi-native filterbanks with no dither, snipped edges and per-bin global mean subtraction, on
normalized samples. Average-linkage clustering merges normalized embeddings at **cosine distance
<= 0.7**. This is a distance threshold, not a cosine-similarity cutoff. No threshold was tuned on
these fixtures. The 180-second input bound produces at most 108 local observations, below the
clustering limit of 128. The original baseline had no coactivity constraint. The development
candidate described in the [coactivity checkpoint](#v55-checkpoint-coactivity-constraints) adds
one; historical measurements below retain their original algorithm and artifacts.

## Sources, code and weights

No model weights, native runtime or source audio are checked into Vults. The re-export's origin
and byte identity were reviewed independently of the engine that publishes it; no engine code
was copied or linked. This experiment does not accept the original Hugging Face contact gate.

| Artifact | Evidence and declared license | Redistribution review |
|---|---|---|
| pyannote segmentation-3.0 | [Original model card](https://huggingface.co/pyannote/segmentation-3.0), MIT; the public ONNX archive carries the CNRS 2022 MIT notice and names the original checkpoint. [Export recipe](https://github.com/k2-fsa/sherpa-onnx/blob/master/scripts/pyannote/segmentation/export-onnx.py). | Retain the copyright and MIT permission notice with redistributed weights. The original checkpoint is gated; only the public export was fetched for this experiment. |
| 3D-Speaker CAM++ | Exact `iic/speech_campplus_sv_zh-cn_16k-common` [model card](https://modelscope.cn/models/iic/speech_campplus_sv_zh-cn_16k-common) and [metadata API](https://modelscope.cn/api/v1/models/iic/speech_campplus_sv_zh-cn_16k-common) declare Apache License 2.0 for the weights. [3D-Speaker code](https://github.com/modelscope/3D-Speaker) is Apache-2.0; the [export recipe](https://github.com/k2-fsa/sherpa-onnx/blob/master/scripts/3dspeaker/export-onnx.py) identifies that checkpoint. | Include Apache-2.0, retain applicable notices and mark modifications/conversion. Check any upstream NOTICE before production distribution; the export release itself does not bundle a separate weights license. |
| `ort` 2.0.0-rc.13 / `ort-sys` | [Rust binding source](https://github.com/pykeio/ort), MIT OR Apache-2.0. Default features disabled; only dynamic loading and API 24 enabled. | Preserve applicable notices. No build-time binary download/copy and no bundled runtime. |
| ONNX Runtime 1.30.0 | [Runtime source/license](https://github.com/microsoft/onnxruntime), MIT, supplied from an existing local runtime cache. | Production still needs a pinned runtime delivery/update strategy and all applicable third-party notices. The local wheel cache is not a Vults distribution channel. |
| `knf-rs` / `knf-rs-sys` 0.3.2 | [Published bindings](https://crates.io/crates/knf-rs), MIT; vendored Kaldi Native Fbank carries Apache-2.0. Source and disabled Python/test CMake paths reviewed locally. | Preserve both license sets and native notices; audit the native wrapper before production. CMake and libclang are needed for this optional dev feature. |

The public export publisher's scripts are Apache-2.0. Its engine crate, speech synthesizers and
GPL code are not dependencies. `cargo deny check` passes with all features; the app's normal
dependency tree contains neither `ort` nor `knf-rs`. A model reference under ADR 0028 is still
required before the app offers any download.

| Downloaded artifact | SHA-256 |
|---|---|
| [Segmentation archive](https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2) | `24615ee884c897d9d2ba09bb4d30da6bb1b15e685065962db5b02e76e4996488` |
| Archive's float `model.onnx` | `220ad67ca923bef2fa91f2390c786097bf305bceb5e261d4af67b38e938e1079` |
| Archive's `LICENSE` | `14d7016ad68e7394d6e6b78d96cc2ae431c905287b89674cfdf021e79e62b8ba` |
| [CAM++ ONNX](https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx) | `f682b514c05d947ee3fa91cd6ec6c5c7543479a128373fa29b1faedccd21fd11` |
| Local Linux x86-64 ONNX Runtime library | `c902c70b3003c0e99fada202f37478c515ae9bba7944c2b2abd0017bae0c82ed` |

## Controlled recorded-speech evidence

[Machine-readable results](../../crates/spike/fixtures/diarization-2026-10-18.json) retain complete
turns, fixture hashes, runtime/model pins, source metadata evidence and rejection outcomes.
They contain no source audio. Measurements use Linux x86-64, Ryzen 7 5700X3D, the dev executable,
CPU sessions with one intra/inter-op thread, warm filesystem and one run per fixture. `totalMs`
includes session/model load and inference, excluding input reading/hashing; peak RSS covers the
whole process (`wait4`). These are short-run observations, not a latency distribution or a
30-minute memory benchmark.

Two public recordings form three controlled fixtures plus a silence control:

- **A:** the 11-second [whisper.cpp v1.8.5 JFK sample](https://github.com/ggml-org/whisper.cpp/blob/v1.8.5/samples/jfk.wav), SHA-256 `59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e`. The [U.S. government recording](https://www.archives.gov/milestone-documents/president-john-f-kennedys-inaugural-address) is public domain.
- **B:** LibriSpeech speaker 1272, recording `1272-128104-0000`, 5.855 seconds, from [the public test subset](https://huggingface.co/datasets/hf-internal-testing/librispeech_asr_dummy) at revision `5be91486e11a2d616f4ec5db8d3fd248585ac07a`. Attribution: Vassil Panayotov, Guoguo Chen, Daniel Povey and Sanjeev Khudanpur, [LibriSpeech / OpenSLR 12](https://openslr.org/12/), [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). Original FLAC SHA-256 `4e25e22555cd16e90edb0a3b49fdcf1fe652b2a1250ab643634db33895c75b41`; decoded mono 16 kHz PCM16 WAV SHA-256 `6ba71ddda339c469a52fa48a375e17e2487a336f4434ae24ef5118637bbcb925`. Decoding retains timing and amplitude; the overlap fixture modifies/mixes the recordings.

Construction uses 16,000-sample silence `Z`, untrimmed source PCM and 44-byte PCM16 RIFF headers:
`single = Z+A+Z`; `alternating = Z+A+Z+B+Z+A+Z`; `overlap = Z+A+Z+mix+Z+B+Z`.
For `mix`, repeat A to B's sample count and use `trunc((A[i]+B[i])*0.5)` per sample to prevent
clipping. The silence control has 160,000 zero samples. These are constructed/repeated excerpts,
not three independent speakers, real meetings or human-labeled speech boundaries. Do not report
DER, WER or general meeting accuracy from them.

| Fixture | Duration | Expected / found voices | Total (load + infer) | Peak RSS | Observation |
|---|---:|---:|---:|---:|---|
| Single | 13.000 s | 1 / 1 | 374 ms | 122,192 KiB | No unknown turns. |
| Alternating A-B-A | 31.855 s | 2 / 2 | 900 ms | 123,576 KiB | A keeps label 0 on return; B keeps label 1, without unknown turns. |
| A + overlap + B | 26.710 s | 2 / 2 | 728 ms | 122,396 KiB | 3.140 s of unknown turns. No identified A turn in the constructed overlap interval [13.000, 18.855] s; B is identified. |
| Silence control | 10.000 s | 0 / 0 | 177 ms | 105,648 KiB | No turns or embedding observations. |

Six unit tests cover average linkage, invalid vectors/budgets, non-duplicated window ownership,
source-clock overlap/unknown turns and preserved speech gaps. Eight executable rejection checks
cover missing arguments, runtime/model fingerprint mismatch, stereo, wrong rate, empty/oversized
WAV and symlink inputs. Linux CI runs the optional feature's tests and Clippy without loading a
runtime, downloading models or reading audio devices.

## V5.5 handoff and release gates

Proceed with a bounded, cancellable production owner and explicit model lifecycle design. Keep
speaker labeling unavailable in the app until these requirements have evidence:

- Native runtime packaging/update policy, notices and the ADR 0028 model reference; hash-checked,
  user-triggered downloads through the existing gate and no implicit runtime/model fetches.
- A production-safe feature-extraction path, worker isolation, cancellation, resource bounds,
  interactive voice priority and model unload/cleanup. Do not reuse the blocking experiment on
  the app runtime or represent the three-minute input bound as full meeting support.
- A human-annotated, consented PT/EN meeting corpus with multiple voices, quiet/short turns,
  noise and overlap, including microphone/system echo. Freeze a disjoint calibration/evaluation
  split and acceptance limits before tuning the 0.7 threshold or changing the embedding policy.
  The coactivity candidate below was developed on calibration failures before that gate; it
  still requires this separate evaluation. Preserve unknown rather than inventing an identity.
- Source-clock alignment and transcript association, plus real 30-minute latency/RSS and
  interruption/cleanup checks. V5.7/V5.8 still own archives, export and the full meetings workspace.

Temporary downloaded weights and generated audio are removed after validation. The user's app,
recordings, microphone/output, voice models and native runtime cache are untouched.

## V5.5 checkpoint: offline admission and text association

2026-10-18. The first V5.5 backend slice is implemented in `vults-voice::meeting`, without
activating labels or downloads in the app. V5.5 as a whole remains open.

- `speaker_models::read_models` admits both exact V5.4 exports as one private pair. It checks
  file type/size, reads at most the pinned 5,992,913 + 28,281,138 bytes plus one EOF probe each,
  and verifies SHA-256 with cancellation checks every 64 KiB. Failure/cancellation drops all
  buffered bytes. Unix opens reject replaced symlinks and avoid blocking on replaced FIFOs.
  Native sessions consume the checked owned bytes through `commit_from_memory`, not a path
  reopened after validation. Buffers drop after both sessions load. The experiment now uses this
  shared admission path; its explicit developer-supplied runtime is still a release gate.
- `speakers::associate` preserves the text and source timestamps exactly. It labels a whole
  segment only if the same known voice covers the entire half-open interval alone. Any gap,
  unknown activity, speaker change or overlapping different voice leaves it unassigned. Duplicate
  activity from the same anonymous speaker does not fabricate overlap. No words are split,
  copied to guessed boundaries or attributed by a percentage threshold. No confidence score,
  personal name or cross-source identity is inferred.
- Association requires equal source-clock durations, non-overlapping ordered text intervals,
  valid turn bounds and limits of four hours, 100,000 turns/segments and 16 MiB of text. A sorted
  event sweep handles unsorted/overlapping turns without a turns-times-segments cross product;
  text attribution then uses binary search. It checks cancellation throughout validation,
  sweep and attribution and returns no partial transcript. Event sorting is bounded by 200,000
  endpoints and is checked immediately afterward, not interrupted inside the standard sort.
  Call separately under each source's owner; equal numeric labels across tracks are unrelated.

Fourteen new voice tests cover file integrity/type/size, post-read path replacement, cancellation,
whole-segment certainty, overlap/unknown/gaps, half-open boundaries, local labels, timeline/budget
rejection and a small exhaustive coverage oracle (4,536 interval cases). The optional spike adds
one regression replay against the frozen V5.4 recorded turn outputs, using authored disposable
probe text; this is not a new ASR accuracy measurement. Linux CI already runs voice tests and the
optional spike feature's Clippy/tests, with no model/runtime downloads or audio capture.

A [native admission smoke record](../../crates/spike/fixtures/diarization-admission-2026-10-18.json)
uses the repository's 3.4-second public-domain speech fixture. Both graphs loaded from verified
memory and produced one anonymous speaker/one embedding observation. Changing one byte in the
embedding graph, with the original length retained, failed before native initialization. This
short smoke run does not revise the V5.4 overlap finding or qualify a production runtime package.
Downloaded experiment weights are removed after the check; no user's audio or model cache is used.

Next: implement the bounded worker/native interruption and interactive scheduling owner, resolve
runtime delivery/notices, and qualify/download models through the existing gate before app wiring.
Keep the V5.4 quality and real-meeting release gates above, including source-clock alignment and
30-minute resource/cleanup evidence. Admission/association cancellation is implemented; aborting
an in-progress native inference is not delivered by this checkpoint.

## V5.5 checkpoint: interruptible bounded native processing

2026-10-18. Native inference is now an optional `vults-voice/diarization` backend, reused by the
existing offline spike command. The app does not enable this feature. Its entry point,
`meeting::diarization::analyze_wav`, is blocking and belongs on the recording worker, under that
caller's consent and temporary-audio cleanup ownership. It takes an already-open source, a
verified model pair, an explicitly admitted runtime, a stop token and content-free progress.
It returns a complete source-local anonymous timeline or no result; it neither saves audio/text
nor calls providers or downloads anything. The existing conservative association API remains the
next caller step, after transcription and source-clock alignment.

The implementation now provides:

- One admitted analysis; competing requests fail before audio reads or native model allocation.
  PCM normalization reuses the meeting reader (8-192 kHz, 1-32 channels, supported integer/float
  WAV encodings). Keep at most 180 seconds / 2,880,000 mono samples (11.52 MB of f32 PCM), rather
  than representing this as full-duration meeting support. Model buffers and session-owned memory
  are additional to PCM; the bounded model pair remains 34,274,051 bytes before session loading.
- Shared native-work admission with interactive Whisper. Each segmentation/embedding operation
  uses fresh ONNX `RunOptions`; a scoped watcher checks stop/foreground demand every 10 ms and
  requests [native termination](https://github.com/pykeio/ort/blob/v2.0.0-rc.13/src/session/run_options.rs).
  The inference call must return before its options, slot or watcher are dropped. A preempted
  operation discards its attempt, lets interactive speech run first and retries that same input
  with fresh options. Cancellation never publishes partial turns or stale late success. Option,
  native and termination failures are retained; a failed termination is not retried forever.
- Reading/loading/analyzing progress with source milliseconds only. Cancellation checks bracket
  normalization, session construction, each window, clustering and final publication. Completion
  after the last window can still be cancelled without a result. Scoped watcher shutdown handles
  unwinding, and slot guards release on failure/panic. Session construction and destruction share
  the priority slot; session wrappers drop owned contexts on completion, cancellation or failure.
- An explicit runtime token prevents engine calls from implicitly searching for a native library.
  Admission requires a trusted immutable absolute file path, independent SHA-256, regular-file
  checks and a 128 MiB hash-read bound. The process pins one canonical path/hash, serializes
  initialization and refuses a runtime environment already committed outside its owner. Models
  still load only from checked bytes. Runtime code remains loaded for the process lifetime; the
  Rust/ONNX singleton cannot be switched or unloaded per meeting.

Termination is cooperative, not a hard deadline: the watcher requests it and waits for native
execution to return. Session construction, filterbank extraction and standard-library sorting do
not provide native abort callbacks; they keep bounded inputs and checks around them. Cancellation
cannot force a hung native operator to stop. A distribution/runtime policy and real interruption
latency evidence are still required before shipping this backend in the app.

Nineteen backend tests now run without a native runtime/model download (six moved from the
original spike plus thirteen new controls/normalization/admission cases). Three additional
opt-in native tests passed locally with the pinned public exports and the existing trusted
ONNX Runtime 1.30.0 library, sequentially:

1. Complete the tracked 3.4-second public-domain speech fixture, then repeat with cancellation
   at the final window and get no result.
2. Observe ONNX's terminate error during an active controlled operation and discard its result.
3. Queue interactive native-work admission, observe it run before the background retry, then
   successfully infer with fresh options. This is scheduler admission evidence, not a captured
   microphone/desktop acceptance test.

To reproduce only these native checks, supply `ORT_DISABLE_TELEMETRY=1` at process startup and
explicitly set `VULTS_DIARIZATION_TEST_RUNTIME`,
`VULTS_DIARIZATION_TEST_RUNTIME_SHA256`, `VULTS_DIARIZATION_TEST_SEGMENTATION` and
`VULTS_DIARIZATION_TEST_EMBEDDING` to reviewed local artifacts, then run:

```sh
cargo test -p vults-voice --features diarization --lib   meeting::diarization::native_tests -- --ignored --test-threads=1
```

CI runs optional voice tests/Clippy plus the spike's frozen-turn replay without enabling those
three native checks or fetching their artifacts. The normal app dependency tree still excludes
`ort`, `knf-rs` and the publishing engine.

[Native processing evidence](../../crates/spike/fixtures/diarization-worker-2026-10-18.json) records
a fresh single-process CLI smoke run: 3.4-second public speech finds one voice at 124,272 KiB peak
RSS; 180-second generated silence finds zero voices at 135,728 KiB. Their total times are 950 ms
and 4,036 ms. The current CLI total includes runtime hashing/initialization, WAV normalization,
session load/inference/destruction; model admission/input fingerprinting happen before that timer.
The historical V5.4/V5.5 admission records use different timing scopes; do not compare them as a
speed regression or improvement. These short checks do not establish real-meeting accuracy,
latency percentiles, a 30-minute resource bound or capture/desktop acceptance. Temporary weights
and generated silence are removed afterward, with the app closed and personal caches untouched.

V5.5 remains open for runtime delivery/notices and model-reference/download integration through
the gate, source-clock alignment and real PT/EN quality/resource acceptance. The native processing
primitive is now implemented; production app wiring and automatic speaker labels remain disabled.

## V5.5 checkpoint: reviewed transactional artifact installation

2026-10-18. The optional `voice/meeting-assets` backend now installs a complete Linux x86-64 GNU
bundle from three fixed public artifacts. It does not enable native analysis or requests in the
app. A new [English model reference](../reference/meeting-models.md), mirrored in Brazilian
Portuguese, records source/code/weights licensing, redistribution notices, sizes, pins and current
release status under ADR 0028. Cargo's permissive bzip2-license exception is restricted to the
reviewed `libbz2-rs-sys` 0.2.5 and its notice is retained with the package.

The installer requires the reviewed manifest fingerprint and a caller-supplied gate on every HTTP
attempt, redirects included. It accepts only fixed GitHub HTTPS release/CDN hosts, disables proxy
and automatic redirect/retry behavior, enforces 180 seconds per artifact and checks streamed size
and SHA-256. Extraction uses bounded Rust decoders and an exact member whitelist, rejecting path
traversal, selected links/duplicates, missing/corrupt data and expansion/count excesses. Its private
stage and process lock retain one attempt. Hash-verified graphs/runtime and complete notices are
published together by atomic directory rename; no partial pair or library is exposed as ready.
An integrity recheck covers all ten files, not just a manifest or file lengths.

Version directories incorporate the manifest fingerprint. Existing versions and loaded runtimes
are preserved; corrupt current-version data reports an error instead of being silently replaced.
A new explicit attempt cleans abandoned stages under the installation lock. Cancellation stops
network waits and bounded extraction; a dropped future signals its blocking work and owns staging
cleanup. The commit point may win a late cancel race. Ordinary failures do not manufacture a
cancelled flag. A committed runtime remains process-pinned, so changing its version needs restart.

The optional Rust developer command provides a concrete review/install caller:

```sh
cargo build -p vults-spike --features meeting-assets --bin vults-meeting-assets
# Read the fixed manifest; no network or writes.
target/debug/vults-meeting-assets review
# Supply that review ID and a new absolute private installation root.
target/debug/vults-meeting-assets install "$MEETING_ASSET_ROOT" "$MEETING_ASSET_REVIEW"
```

This command has a twelve-request invocation cap and local content-free outcomes. It is not the
app's consent/quota/trace adapter. The app has no install command/button or automatic caller for
this backend; that owner and UI are the next integration step. The normal app dependency tree
still excludes this optional parser/download feature and the native ONNX engine.

The runtime audit also closed the startup-privacy requirement: app and native-spike entry points
set the process flag before workers, and the native loader rejects missing startup privacy and
sets its API telemetry option false. This requirement applies to any explicitly supplied runtime,
including older development-library paths. Reproduce native tests only with the documented
startup environment; historical recordings remain historical evidence, not privacy qualification.

[Recorded installation evidence](../../crates/spike/fixtures/meeting-assets-2026-10-18.json) contains
pins and content-free outcomes: ten installed files, six admitted HTTP attempts, zero HTTP on
cached revalidation. The installed official CPU runtime 1.31.0 passes all three native interruption
checks, including the tracked public-domain speech fixture. Twelve offline installer tests cover
streams, cancellation/deadlines, rejected gate/review, archive safety/budgets, lock/recovery and
preserved existing data. No personal audio/profile or previous user runtime/model cache was used;
all new downloads, installed test bundles and generated staging data are removed after validation.
Real PT/EN quality/source-clock acceptance and production app activation remain pending.


## V5.5 checkpoint: app-owned artifact installation

2026-10-18: **Dictation > Voice** now offers a collapsed experimental installer. Nest Meetings
links to the existing Settings owner; all four installer commands reject Nest/Island callers.
An explicit review verifies local files off the UI thread, binds a one-use manifest token to the
request ledger and expires after one minute. A separate consent checkbox and install click start
one bounded job. Progress names only the current artifact; cancellation waits for the attempt to
finish and drop its staging owner. Cleanup is attempted; leftovers are recovered on the next
explicit installation attempt.
Installation continues across navigation, while leaving an unconfirmed review revokes it. Quitting
signals cancellation; abandoned staging is recovered on the next explicit installation attempt.

`app/src/gate/meeting_assets.rs` durably reserves every HTTP GET, including redirects, within
24 attempts per rolling 24 hours. A restart/history clear cannot erase reservations. Ledger
replacement invalidates the old review, storage errors fail closed, and the generic connector
switch cannot open this gate. Started and finished local spans contain only artifact IDs, outcome
and timing. No automatic retry, resume, update check or analysis activation is added.

The normal app now includes the transactional artifact/parser feature, independently of the
native diarization engine. It still does not link/load that engine or use speaker labels. The
prior public-download/native-runtime evidence remains a developer-backend result, not a claim
of real PT/EN meeting quality.

Validation: 1,174 workspace tests passed (9 opt-in cases ignored), the optional voice suite passed
89 cases (4 ignored), and all 8 eval cases passed. Workspace and optional-feature Clippy, formatting,
Cargo deny and repository guards passed. Sixteen renderer scripts cover the production views,
including the new installer's consent, navigation, late replies, cancellation and reconnection.
The AGENTS.md pre-commit list also names `check-task-actions.mjs`, which is absent in this branch;
no replacement script or passing result is invented for it.

The isolated native app rejects all four foreign-webview installer commands, rejects stale or
unconfirmed reviews, refuses an exhausted disposable quota before HTTP, cleans staging and
cancels an unconfirmed review. Black/Light review controls fit at 1180 x 820 and 480 x 360 without
horizontal overflow; package details remain a disclosure. Controlled native status responses
exercise progress/verified-state rendering without downloads, including three translations.
[Content-free evidence](../../crates/spike/fixtures/meeting-assets-app-2026-10-18.json) distinguishes
real backend assertions from controlled renderer states. Personal audio/profile/cache is unused;
isolated processes and temporary caches are removed after validation.

Next: qualify real PT/EN speech and independent source clocks before connecting native analysis
to meeting transcription. Summaries, saved meetings, export and realtime conversation remain open.


## V5.5 checkpoint: source-bound offline quality scoring

2026-10-18: `vults-meeting-eval` now scores supplied reference/prediction JSON without audio,
models, native runtime, network or app activation. Schema-1 clocks bind recording/source IDs,
WAV SHA-256 and duration; equal-duration wrong-source/hash data is rejected. Per-source input
limits are one MiB, 4,096 turns per side, eight known voices per side and 180 seconds. Linux
opens reject symlinks and non-regular files, including FIFOs without waiting for a writer.

The fixed profile retains overlap and uses a zero collar. An exact subset solver finds one
one-to-one global anonymous-label mapping, then reports missed, false-alarm and confused speaker
milliseconds, correct speaker time and unknown activity. Duplicate activity for one label is
merged. Null prediction activity is one unmatchable stream and is never optimized into a person;
multiple null turns cannot establish how many unknown people spoke. Silence-only references
retain their false alarms with no invented finite DER. Canonical input SHA-256, annotation
provenance, language and split tags remain in the report. No acceptance threshold is introduced;
`productionQualified` is always false, even for a declared human reference or perfect score.

The optional native spike accepts `--reference` to reuse this scorer. It checks the actual WAV
hash before runtime initialization and normalized duration before publishing. Hashing/inference
retain one open descriptor; replacement of the pathname does not switch input, and a second
hash rejects changed content. This is not an immutable snapshot against adversarial transient
writes. New `totalMs` measurements include that final verification read; historical runtime
measurements above keep their original method. The app still has no native speaker-analysis
linkage and no new recording, permission or request path.

Twelve unit cases include 729 rectangular matching matrices checked against a brute-force oracle
and 36 independent framewise tape-pair comparisons. Three optional spike cases cover frozen-turn
replay, actual-hash/duration binding and descriptor/content mutation. One executable integration
case checks both language tags, wrong-clock refusal, usage errors and an output that cannot
qualify production. The complete regression has 1,187 workspace tests passing (9 ignored),
21 all-feature spike tests passing, 8 eval cases, workspace/feature Clippy, formatting, Cargo deny
and repository source/language/layer/network guards. No renderer change was made or native audio
inference rerun in this slice; the app remains closed.

[Recorded scorer evidence](../../crates/spike/fixtures/meeting-eval-2026-10-18.json) distinguishes
invented timelines from human speech. EN/PT fixtures exercise metadata, not spoken language:
one preserves permuted identities with DER 0, and another retains the same two speaker counts
but has 2,000/5,000 confused speaker-ms (DER 0.4). Their placeholder audio hash is not a recording.
The [bilingual evaluation reference](../reference/meeting-evaluation.md) gives reproduction,
profile, provenance caveats and limits. No personal audio/profile/cache was used.

Next: obtain/review a consented, human-annotated PT/EN corpus, freeze disjoint calibration/evaluation
splits and quality limits, and then measure native results. Physical source-clock/echo and long
meeting resource acceptance remain open; this measurement tool is not production qualification.


## V5.5 checkpoint: first public human calibration

2026-10-19: the existing native spike and scorer measured the first 120 seconds of AMI IS1000a
and the explicitly public C-ORAL-BRASIL bfamdl02 sample against their human annotations. No new
parser, download path, model setting, native algorithm or app activation was introduced. The
selection, original/cropped WAV and annotation/reference hashes, zero-collar overlap profile,
fixed 0.7 threshold and provisional DER target 0.20 were frozen before inference. These are
calibration-only observations, not a recording/speaker-disjoint evaluation set.

Both fail that provisional diagnostic target: AMI has 3 annotated/found voices but DER 82.12%
(5,534 missed, 15,329 false-alarm and 2,631 confused speaker-ms / 28,610 reference speaker-ms).
C-ORAL has 2 annotated voices collapsed into 1, with DER 39.79% (26,617 missed, 641 false-alarm
and 26,015 confused speaker-ms / 133,901 reference speaker-ms). The Portuguese reference includes
utterance pauses; the English setup uses word intervals. Do not compare these percentages as a
language ranking or retune the threshold to improve them. Simultaneous-observation clustering
constraints remain an identified review point, not a proven fix in this slice.

The actual 22.05 kHz stereo Portuguese crop and 16 kHz mono English crop both retain their source
clock through the production normalization path, and source/normalized-duration admission passes.
Native and standalone reports are identical and agree with an independent one-millisecond raster
and exhaustive label-permutation oracle. One CPU run per crop, one intra/inter-op thread, produced
3,859/8,950 ms native totals and 130,344/155,668 KiB whole-child peak RSS on Ryzen 7 5700X3D.
No performance distribution, physical two-device synchronization or 30-minute acceptance is claimed.

The [bilingual reference](../reference/meeting-evaluation.md#human-calibration-on-2026-10-19) records
public source/license attribution, exact preparation and annotation differences. The
[content-free evidence](../../crates/spike/fixtures/meeting-human-calibration-2026-10-19.json)
contains the pre-inference manifest and aggregate reports, not audio, transcripts or source
annotation turns. C-ORAL's public sample is CC BY-NC-SA 4.0 and was used for this local noncommercial
research check; AMI corpus material is CC BY 4.0 with the pinned BUT setup. No contact form or
personal profile was used. Temporary public files and the fresh disposable model/runtime bundle
are removed afterward; existing user and shared Cargo caches are preserved.

Validation: both real native invocations, standalone comparisons and independent oracles pass;
21 all-feature spike tests, formatting and repository brand/English/i18n/layer/network/README
guards pass. There is no renderer change; the user's app remains closed. The result is a no-go
for automatic labels. Next: recording/speaker-disjoint human evaluation and review of coactive
merge constraints before quality tuning; echo, noise, quiet turns and long-session gates remain.

## V5.5 checkpoint: coactivity constraints

2026-10-19. The development-only native candidate now forbids merging local embedding
observations active together in an owned powerset frame. A symmetric infinite pair distance
keeps the constraint through every average-linkage group merge. Only known embedded observations
contribute constraints; unknown/short voices remain unknown. No dependency, model pin, embedding
policy, fixed 0.7 threshold, cancellation owner or app linkage changed. The CLI declares
`coactivityConstraints: true`; the original evidence remains intact.

This is a correction motivated by the preceding calibration failure, before the originally
planned disjoint evaluation gate. It cannot satisfy that gate. The same two frozen WAV/reference
fingerprints were reproduced and measured once again: AMI prediction/quality is identical
(3 voices, DER 82.12%); C-ORAL now separates 2 voices, with DER 17.29% instead of 39.79%. Its
confusion falls from 26,015 to 1,428 speaker-ms and missed activity from 26,617 to 17,880, while
false alarms rise from 641 to 3,847. The Portuguese candidate meets the original provisional
diagnostic 0.20 target; the English candidate fails. Predicted coactivity can be wrong, so this
does not establish speaker identity or release quality.

The [bilingual comparison](../reference/meeting-evaluation.md#coactivity-correction-on-2026-10-19)
and [content-free evidence](../../crates/spike/fixtures/diarization-coactivity-2026-10-19.json)
retain aggregate errors/resources, unchanged model pins, original artifact hash and limitations.
Native and standalone scores match an independent millisecond raster/exhaustive mapping oracle.
One CPU run per crop produced 3,847/9,029 ms native totals and 130,324/148,248 KiB whole-child
peak RSS; no performance distribution is claimed. Public sources, attribution and annotation
conventions remain those of the original calibration. No source audio, text or annotation turns
are redistributed; disposable files/models/runtime are removed after validation.

Validation: the new identical-vector regression proves direct and intermediate forbidden merges
stay separate and invalid/cancelled constraints publish no labels. The optional voice suite
passes 90 tests (4 ignored), the spike suite 21, and all 3 explicit native completion/cancellation/
preemption checks pass. Formatting, feature Clippy and repository guards pass. There is no renderer
change and the app remains closed. Next: freeze recording/speaker-disjoint human evaluation
material and acceptance limits before further tuning; physical source alignment, echo, noise,
quiet turns and long-session resources remain open. Automatic labels stay unavailable.

## V5.5 checkpoint: pause correctness and false-alarm diagnostics

2026-10-19. The development worker no longer fills gaps up to 17 ms when merging turns.
Touching/overlapping activity still merges, but real timestamp gaps cannot manufacture continuous
speaker coverage for text association. The regression fails before the fix and checks known/
unknown activity, gaps of 1/16/17/1,000 ms, adjacent frames and text coverage afterward. No minimum
turn-length or energy filter was added. Models, coactivity constraints and distance 0.7 stay fixed.
The offline report now partitions false alarms into reference-absent activity and excess overlap,
without changing total DER/matching; duplicate and unknown activity are covered by a scorer test.

The frozen pause experiment reproduces the original two calibration predictions. IS1009a adds
17 missed speaker-ms (DER 61.06% to 61.09%); the controlled Portuguese noise case also adds 17.
Its no-regression quality gate therefore fails. Retain the gap fix only for development timeline/
association correctness, not a claimed quality improvement. A separately frozen stronger grouping
candidate forbids every pair of known local slots, even without coactivity. It regresses IS1000a
from DER 82.12% to 90.64% and was removed, despite improving individual other recordings. Do not
cherry-pick those gains or activate labels. IS1009a moved from evaluation to calibration once
its collapsed voices motivated that candidate; ES2004a was preselected before the second candidate.
The retained worker finds 2/4 voices in IS1009a and 1/3 in ES2004a, with DER 61.09% and 31.33%.
Distinct source recordings do not establish speaker identity or model-training independence.

The new counters show different annotation-relative error profiles: IS1000a has 15,329
reference-absent/0 excess-overlap speaker-ms; Portuguese has 649/3,198. Acoustic review is still
needed because missing annotation is not verified silence. Six controlled gain-0.1, 15 dB uniform
noise and 200 ms/gain-0.35 echo variants use propagated intervals, not new human annotations.
Echo raises Portuguese DER from 17.29% to 31.62%. They do not establish physical echo cancellation,
clock alignment, held-out Portuguese accuracy or robustness of naturally quiet/short voices.

See the [bilingual refinement reference](../reference/meeting-evaluation.md#pause-and-diagnostic-refinements-on-2026-10-19)
and [content-free evidence](../../crates/spike/fixtures/diarization-refinements-2026-10-19.json)
for frozen plans, source/reference/model hashes, rejected outcomes, retained reports and controlled
transformation recipes. Ten final native reports match standalone scoring and independent
millisecond raster/exhaustive mapping, including both new counters. Reconstructed former gap
filling exactly matches three separate native baseline runs. Validation passes 90 voice tests,
22 spike tests, 3 explicit native checks, formatting, feature Clippy and repository guards.
No renderer/app activation; disposable downloads, annotations, audio and models/runtime are removed.
Next: acoustic/reference review of English false alarms, independently evaluated speech/overlap
detection and speaker grouping. Independent Portuguese human data, real source echo/alignment,
quiet/short turns and long-session acceptance remain open.

## V5.5 checkpoint: decoder ties and reference definitions

2026-10-19. Exact powerset ties now keep the first maximum, matching the original hard argmax
decoder. The previous last-maximum comparator preferred overlap on a seven-way tie and treated
signed zeros differently. A pure regression fails before the fix and checks all maximum pairs,
unique maxima and one-float-step near ties. Native shape/finiteness checks still precede decoding;
no confidence tolerance, threshold, model pin or grouping policy changed. CLI metadata declares
`argmaxTieBreak: first`. All four primary prediction fingerprints and complete reports reproduce
the preceding checkpoint, so no measured accuracy improvement is claimed on these clips.

A frozen paired-reference audit reuses the same known 120-second sources, keeping word-only
primary scoring. The pinned BUT `word_and_vocalsounds` annotations cover zero additional false
alarms on IS1000a/IS1009a and 123 speaker-ms on ES2004a (3,244 to 3,121). They do not explain
most false alarms, and their documented annotation inconsistencies prohibit treating them as
verified acoustic truth. Changed reference time/mapping gives diagnostic DER 83.22%, 61.09%
and 35.41%; these are not changes in the model's performance. No new Portuguese or held-out
recording was added. Original licensing, attribution, crop clocks and primary profile remain.

The [bilingual audit](../reference/meeting-evaluation.md#decoder-ties-and-reference-audit-on-2026-10-19)
and [content-free evidence](../../crates/spike/fixtures/diarization-reference-audit-2026-10-19.json)
record the frozen plan, model/input/reference pins, source contracts, unchanged primary reports
and alternate-reference diagnostics. Seven standalone scores match independent millisecond
raster/exhaustive mapping; 91 voice tests, 22 spike tests, 3 explicit native checks, formatting,
feature Clippy and repository guards pass. No renderer or app activation. Disposable public
audio/annotations/models/runtime are removed afterward, with the user/shared caches preserved.
Next: separately evaluate the existing Silero detector as a bounded diagnostic candidate before
considering any speech filter, preserving short/quiet speech and freezing acceptance material.
English missed/merged voices, Portuguese overlap/echo, independent Portuguese human evaluation,
physical source alignment and long-session acceptance remain open.

## V5.5 checkpoint: existing speech-detector diagnostic

2026-10-19. The spike's explicit `--reference ... --vad ABSOLUTE_MODEL` option compares unchanged
diarization with a mask from the existing catalog-pinned Silero model and existing 200 ms margin.
Default invocations have `vadDiagnostic: null`; no app feature, detector threshold, dependency,
dictation behavior or baseline labeling changes. The catalog hash is shared with caller admission
and remains omitted from renderer serialization. Linux native loading uses the retained verified
descriptor; model/source rehashing detects persistent edits, not an immutable snapshot.

The optional backend reuses the bounded source reader and one-analysis admission. VAD load,
inference and destruction share one background-admitted scheduler slot. Silero has no native
abort callback, so cancellation discards results after its bounded whole-source call completes;
this is not production preemption. Pure intersection checks cover gaps, overlap, unknown labels,
one-millisecond turns and the output budget. Real default VAD fixtures retain their existing
speech/pause/silence/noise behavior. No UI or installer behavior was changed.

A frozen profile (existing 0.50/0.35 thresholds, smoothing 0.35, 250 ms speech/400 ms silence,
200 ms margin, one CPU thread) measures five human clips, six controlled variants and a controlled
200 ms public-domain speech fragment. Ten known baseline reports exactly reproduce prior results.
TS3003a first120 was chosen before candidate output as an additional recording-disjoint English
diagnostic; speaker identity and model-training independence remain unestablished. No new
Portuguese human source was added. All 24 reports match independent millisecond raster/exhaustive
mapping and standalone scoring; each candidate interval retains an original label and lies inside
a baseline turn. No threshold or margin sweep was used.

Reject general speech masking: all 12 cases add missed speech. IS1000a false alarms fall from
15,329 to 1,368 speaker-ms, but misses increase by 5,235 and a reference turn loses all coverage.
Portuguese DER worsens 17.29% to 17.57%. New TS3003a worsens 17.49% to 31.26%, adds 12,875 missed
speaker-ms and loses four reference turns. Quiet/noise/echo controls also lose speech; the 200 ms
fragment loses its entire 170 ms baseline coverage. Better aggregate DER on some cases cannot
override the frozen speech-preservation gate. The failed candidate stays diagnostic-only.

The [bilingual VAD reference](../reference/meeting-evaluation.md#existing-speech-detector-evaluation-on-2026-10-19)
and [content-free evidence](../../crates/spike/fixtures/diarization-vad-evaluation-2026-10-19.json)
preserve frozen selection/criteria, all outcomes, source/model pins, controlled recipes and licenses.
Validation passes 91 optional voice tests (5 ignored), 23 spike tests, 4 explicit native checks,
57 default voice tests with real Silero fixtures, feature Clippy, formatting and repository guards.
Three malformed/untrusted VAD admissions reject before runtime loading, and an ordinary native
run confirms the option stays off. Disposable audio, annotations, controls and models/runtime are
removed afterward; app closed, user/shared caches preserved. Next: independently evaluated
meeting-specific detection that protects short/quiet speech and supports production cancellation,
without retuning the shared dictation profile. Speaker grouping, overlap/echo, independent
Portuguese human evaluation, physical source clocks and long-session acceptance remain open.

## V5.5 checkpoint: meeting-frame speech preservation

The explicit offline `--vad-frames` candidate selects raw Silero frames at probability >=0.5,
without smoothing or speech/silence dwell, then applies the same 200 ms margin. An explicit
`SpeechProfile` selects this candidate or the unchanged dictation profile; `--vad` remains
reproducible. Shared inference validates native probabilities for both decoders. No app feature,
model, dependency, installer, default speaker decoding or Chat/dictation timing changes.

The pre-inference plan freezes one candidate, strict no-regression gates and 13 sources: all
12 previous cases are now known calibration, plus preselected EN2002a first120 as one new
English recording. Historical split fields are preserved without treating known clips as
held-out evaluation. Speaker/model-training independence, independent Portuguese human sources,
physical capture/clocks and long-session acceptance remain unestablished.

The short fragment keeps all 170 ms of its baseline coverage, previously erased by dictation's
profile. That is the only passing case. All six human clips and six controlled stress cases
add missed speaker-ms; general hard masking is rejected again without a parameter sweep.
New EN2002a DER worsens 35.90% to 37.26%, adding 2,987 missed speaker-ms. The Portuguese quiet
control worsens relative to both baseline and the previous candidate. The utterance-reference
pause caveat remains; English word-coverage failures independently prevent acceptance.

See the [bilingual reference](../reference/meeting-evaluation.md#meeting-frame-speech-profile-on-2026-10-19)
and [content-free evidence](../../crates/spike/fixtures/diarization-meeting-frames-2026-10-19.json).
All 12 known baseline reports remain identical; 26 reports match standalone and independent
millisecond raster/exhaustive mapping checks, with same-identity subset/activity-conservation
proofs. Old-profile English/short-control reports reproduce and ordinary diagnostic output stays
null. Four invalid admissions reject before native runtime loading. Validation: 92 feature voice
(5 ignored), 23 spike, 4 explicit native and 57 default voice tests with real Silero fixtures;
all-feature Clippy, formatting and repository guards pass. Disposable public sources, controls,
annotations, models/runtime and logs are removed; app closed, user/shared caches preserved.
Next: independently validated detection/overlap work and acoustic review of remaining speech
loss, with production cancellation and the other real-meeting acceptance gates still open.
Do not continue hard-mask threshold/margin sweeps or activate automatic labels from this result.

## V5.5 checkpoint: millisecond edge validity

Clipping a frame to source/ownership bounds can leave sample activity whose converted start and
end coincide. `merge_turns` now removes these zero-duration turns before merging, without rounding
up, filling gaps or changing positive-duration turns. Previously isolated collapsed edges could
invalidate the entire text-association/scoring result. Sources normalizing to less than one
millisecond now reject before reading progress/model-session construction, shared by the native
analysis and both offline VAD profiles.

Two regressions fail before the fix and pass afterward: known/unknown edge fragments with valid
one-millisecond text association, and just-below/exactly-one-millisecond source admission at
8/16/48/192 kHz, including stereo. Passing checks: 94 feature voice (5 ignored), 23 spike,
all-feature Clippy, formatting and repository guards. See the [bilingual clock reference](../reference/meeting-evaluation.md#millisecond-timeline-edges).
No native/model/audio experiment was rerun; no new quality result or speaker-label activation is
claimed. Temporary check logs are removed; app closed, user/shared caches preserved. Next remains
independent detection/overlap and acoustic speech-loss review, production cancellation and the
other real-meeting acceptance gates; rejected masks stay diagnostic-only.

## V5.5 checkpoint: operation preparation cancellation and priority

The shared ONNX controller now rechecks stop/foreground requests after per-operation options
construction. Cancellation skips inference; pending interactive work releases the prepared
options/slot and retries preparation afterward. Arrivals during an active inference still use
the existing native abort/discard/retry path. Session construction and Silero whole-source calls
remain non-abortable; no change to speech thresholds, clocks, grouping or app activation.

Two deterministic preparation races fail before the fix and pass afterward. Passing checks:
96 feature voice (5 ignored), 23 spike, 3 explicit native checks, all-feature Clippy, formatting
and repository guards. Native checks cover completion/late cancellation, active termination and
foreground-first retry using the existing catalog-pinned bundle, tracked fixture and synthetic
input. See the [bilingual scheduling reference](../reference/meeting-evaluation.md#native-operation-preparation).
Temporary models/runtime/logs are removed afterward; app closed, personal/shared caches preserved.
This is scheduler correctness, not a new human quality result. Detection/overlap, acoustic review,
physical source clocks, production integration and long-session acceptance remain open.

## V5.5 checkpoint: shared VAD native ownership

Interactive Silero detection and destruction now share the native-work scheduler, matching the
existing constructor and Whisper ownership. The fix covers Chat's end-of-speech watcher, final
trimming and dictation's speech-presence check at their common VAD owner. The offline background
path retains one slot throughout its raw context lifetime and reuses the locked decoder without
recursive admission. No threshold, hysteresis, margin, source clock or app activation changes.

A native regression fails before the fix for both detection/freeing bypasses and passes afterward
while holding only an idle slot; no concurrent native-engine crash experiment is performed.
Both offline speech profiles pass their native check without deadlock. Passing checks: 96 feature
voice (6 ignored), 57 default voice (2 ignored), 23 spike, 2 explicit native tests, all-feature
Clippy, formatting and guards. Both normal voice suites use the real catalog-pinned Silero model
and existing speech/pause/silence/noise fixtures. See the [bilingual ownership reference](../reference/meeting-evaluation.md#shared-speech-detector-ownership).
App closed, temporary model/logs removed, personal/shared caches preserved. This closes a native
serialization gap, not human quality acceptance; independent detection/overlap, physical clocks,
production integration and long-session qualification remain open. The diagnostic's whole-call
cancellation limitation remains unchanged, and failed speech masks stay off in the app.

## V5.5 checkpoint: shared WAV clock admission

Move the normalized one-millisecond minimum from speaker-only admission into the shared WAV
reader, before resampler allocation. Transcription, native analysis and both offline VAD profiles
now reject unrepresentable source clocks consistently. This closes the transcription path that
could report an empty zero-duration success. Remove the duplicate speaker guard; valid source
clocks and the existing clipping/normalization/cancellation rules remain unchanged.

Two new regressions fail before/pass after: transcription skips progress/decode on invalid clocks,
and the shared reader covers seven rates (including non-integral millisecond sample counts),
mono/stereo and float32/PCM16/24/32 at/below the minimum. Existing chunk-coverage/non-finite fixtures
use the minimum valid duration to retain their original checks. Passing: 98 feature voice
(6 ignored), 59 default voice (2 ignored), 23 spike, all-feature Clippy, formatting and guards.
See the [updated bilingual clock reference](../reference/meeting-evaluation.md#millisecond-timeline-edges).
Synthetic inputs only; no native inference, model/audio download or new human quality claim.
App closed, temporary logs removed, personal/shared caches preserved. Detection/overlap, physical
clocks, production integration and long-session acceptance remain open; masks stay diagnostic-only.

## V5.5 checkpoint: buffered WAV reads

Wrap the common WAV input in stdlib BufReader so raw meeting files are not read once per scalar
sample. The same normalization/clock/validation paths remain in use for transcription, native
analysis and both offline VAD profiles; PCM memory limits and source-file ownership remain.

The counting-reader regression fails before the fix with 96,017 backing reads for one second
of stereo 48 kHz float32. It passes afterward under a 407-call bound, with exact reference
mono/resampled samples and the complete 16,000-sample clock. This measures I/O calls on synthetic
input, not wall-time/device/long-session qualification. Passing checks: 99 feature voice
(6 ignored), 60 default voice (2 ignored), 23 spike, all-feature Clippy, formatting and guards.
See the [bilingual buffered-input reference](../reference/meeting-evaluation.md#buffered-wav-input).
No native/model/audio experiment or app startup; temporary logs removed, personal/shared caches
preserved. Human quality, physical source clocks, integration and long-session acceptance remain
open; rejected speech masks stay diagnostic-only.

## V5.5 checkpoint: ready-stream download deadline

The model installer's body reader now checks its absolute deadline before polling and after
receiving a result, including EOF. A ready stream can bypass `timeout_at`'s elapsed timer;
previously an already-expired, valid stream could still succeed. The existing 180-second
artifact deadline, cancellation, hashes, request quota and staging cleanup remain unchanged.
Local file flush/sync and extraction are separate work, not a hard wall-time guarantee.

The new regression reproduces the old false success and now rejects before writing data or
reporting progress. The all-feature voice suite passes 100 tests (6 explicit native/corpus
checks ignored). No HTTP request, model download, native-quality experiment, renderer change
or speaker-label activation is involved. See the [bilingual installer reference](../reference/meeting-models.md#installation-behavior).
The user's validation app and development server were stopped; personal/shared caches remain.
Independent human quality, physical clocks, integration and long-session acceptance stay open.

## V5.5 checkpoint: HTTP admission deadline and cancellation

Each initial/redirect HTTP attempt now rejects expiry before reserving quota. The admitted
trace owner rechecks cancellation/expiry after the durable gate and after response receipt,
before another request can start or a response can be accepted. A reservation already made
remains charged; its trace closes as cancelled or failed, never leaving a started attempt open.
The existing body deadline, request limit, hashes and staging/cleanup ownership stay in place.

Two deterministic regressions fail before the fix and pass afterward: expired admission never
calls the gate, and cancellation arriving during admission prevents a request token and closes
exactly one cancelled trace. The all-feature voice suite passes 102 tests (6 native/corpus
checks ignored). No HTTP, model download, renderer change or native speech experiment is needed.
The [bilingual installer reference](../reference/meeting-models.md#installation-behavior) records
the boundary. App/server stay stopped; quality, source-clock, integration and long-session
acceptance remain open, with automatic labels still disabled.

## V5.5 checkpoint: extraction cancellation boundaries

Shared artifact-member extraction now checks cancellation before creating output, after each
read (including EOF) and after file synchronization. Archive extraction also rechecks after
its final zero header: that marker can end iteration without another cancellation-aware read.
Cancelled work returns an error to the existing staging owner; cleanup and version preservation
keep their existing ownership. Blocking filesystem calls remain cooperatively cancellable only
after they return, not forcibly interrupted.

Two regressions fail before the fix and pass afterward. They cover pre-cancelled output creation,
cancellation during member data/EOF reads and cancellation at the final archive marker. Existing
private-stage cleanup, locking and installed-version preservation checks also pass. The full
all-feature voice suite passes 104 tests (6 native/corpus checks ignored), and the expanded
member-read regression passes separately. The [bilingual installer reference](../reference/meeting-models.md#installation-behavior)
records the boundary. No HTTP, model download, renderer change or new speech-quality result;
app/server stay stopped and automatic labels remain disabled. Real quality, source clocks,
integration and long-session acceptance remain open.

## V5.5 checkpoint: cached artifact verification cancellation

Pre-cancelled readiness/file checks now stop before filesystem lookup/opening. The common
cached-file hash reader checks cancellation after every read, including EOF, and before returning
success. This prevents an absent-bundle result for already-cancelled work and a verified result
when cancellation arrives in the final read. Regular-file, exact-size and SHA-256 admission stay
unchanged; the reader helper lets tests inject that final-read race without personal models.

Both new regressions fail before the fix and pass afterward. They cover pre-cancelled missing
paths, cancellation during data/EOF reads and non-cancelled valid, truncated, oversized and
equal-size corrupt input. The full all-feature voice suite passes 106 tests (6 native/corpus
checks ignored). See the [bilingual installer reference](../reference/meeting-models.md#installation-behavior).
No HTTP, model download, renderer change or new speech-quality evidence is involved. App/server
stay stopped; quality, source clocks, production integration and long-session acceptance remain
open, and automatic labels stay disabled.

## V5.5 checkpoint: synchronize staged directory before publication

After verifying and synchronizing member files, commit now synchronizes the staging directory
before its final cancellation check and atomic rename. The existing installation-root sync
still follows publication. A failed staging-directory sync returns through the existing cleanup
owner before publishing; cancellation arriving during that blocking call is observed afterward.
This closes the missing member-directory persistence step described by [Linux fsync](https://man7.org/linux/man-pages/man2/fsync.2.html).

The freshly rebuilt developer installer completes one disposable public installation with the
unchanged catalog/review `ca4794c24dc45f6a41d99e4559070f3edd421d9b22de6100ac6c27ee2018fcfc`:
six gated HTTP attempts and ten verified files. A second invocation revalidates all ten with
zero HTTP attempts. No staging directories remain, and the entire disposable bundle is removed.
This exercises successful directory synchronization/publication on the local filesystem; it
does not simulate power loss, physical storage faults or prove a hardware durability guarantee.

Passing checks: 106 all-feature voice tests (6 native/corpus checks ignored), feature Clippy,
formatting and repository guards. The [bilingual installer reference](../reference/meeting-models.md#installation-behavior)
records the ordering. No native inference, renderer change or speaker-label activation; app/server
stay stopped and personal/shared caches remain. Human quality, source clocks, integration and
long-session acceptance stay open.

## V5.5 checkpoint: private installation lock admission

Before locking or deleting abandoned staging data, the installer now validates the opened lock's
descriptor as a regular file owned by the current user, mode 0600 and with exactly one hard link.
This matches the existing temporary-audio lock invariant. Previously only regular-file type was
checked. Invalid locks are preserved and refused, without chmod, unlinking or automatic repair.

The new regression fails before the fix on an accepted mode-0644 lock. Afterward it covers
0644/0666 and a hard-linked lock, proving existing lock bytes and abandoned-stage contents are
unchanged on refusal. A valid 0600 lock still admits one stage, rejects concurrent admission and
preserves its contents. Existing normal recovery/version-preservation tests pass. The full
all-feature voice suite passes 107 tests (6 native/corpus checks ignored). See the [bilingual
installer reference](../reference/meeting-models.md#installation-behavior).

No HTTP, model download, renderer change or speech-quality experiment. App/server stay stopped;
human quality, physical source clocks, integration and long-session acceptance remain open, and
automatic labels stay disabled.

## V5.5 checkpoint: download cancellation at ready IO boundaries

Downloads now reject pre-cancelled work before client/temporary-file setup and recheck after
receiving a body chunk, before write progress, and after file flush/sync. A ready chunk that
sets cancellation while being polled is discarded without writing or publishing progress.
Already-started filesystem IO remains cooperative: it can finish before the next check.
The existing file-path cancellation helper moves onto Cancel and is shared by file and network
owners; error text, request quota, deadline, hashes, staging ownership and app linkage stay fixed.

Two deterministic regressions fail before the fix and pass afterward. They prove a pre-cancelled
fetch creates no staging file/gate attempt/trace/progress, and a stream cancelling from its ready
chunk poll produces no data or progress. Existing pending-stream, timeout, quota, extraction,
verification and lock checks continue to pass. The full all-feature voice suite passes 109 tests
(6 native/corpus checks ignored). See the [bilingual installer reference](../reference/meeting-models.md#installation-behavior).

No HTTP, model download, renderer change or new speech-quality evidence. App/server stay stopped;
human quality, source clocks, integration and long-session acceptance remain open, with automatic
labels still disabled.

## V5.5 checkpoint: cross-process interruption and recovery

A new automatic regression starts another copy of the test executable with a private fixture
root. The child holds a real Stage/file lock and a partial download while the parent verifies
that a competing admission leaves the active stage untouched. The parent then sends SIGKILL,
asserts the termination signal and orphaned directory, and proves a fresh admission reacquires
the lock, removes that orphan and preserves a synthetic previous-version marker. Dropping the
recovered stage also removes its own directory.

Atomic readiness publication avoids a partially written handshake; a ten-second readiness bound
and kill/reap guard clean up the child on test failure. This adds coverage for existing runtime
behavior. There is no production/backend change, model download, HTTP request or power-loss
simulation. The [bilingual installer reference](../reference/meeting-models.md#validation-status)
records the evidence separately from storage-fault and human meeting acceptance.

The all-feature voice suite passes 110 tests (6 native/corpus checks ignored), including the new
cross-process case. App/server stay stopped and disposable fixtures/child processes are cleaned;
quality, physical source clocks, integration and long-session acceptance remain open, with
automatic labels still disabled.

## V5.5 checkpoint: cancellation during abandoned-stage recovery

The lock-owned recovery loop now checks cancellation before iteration, after each directory
entry arrives, after reading entry metadata and before reporting completion. Pending abandoned
entries are left for the next explicit attempt. Recursive filesystem removal already underway
can finish before cancellation is observed; no hard IO deadline or forced interruption is added.
The existing private-root/lock checks, directory/symlink selection and installed-version
preservation remain unchanged.

The new deterministic regression fails before the fix and passes afterward: it cancels as the
second entry arrives, proving the first stage is removed, the second stays intact and a previous
version marker survives. A pre-cancelled recovery does not poll its iterator; a fresh explicit
retry removes the remaining stage and preserves the marker. The full all-feature voice suite
passes 111 tests (6 native/corpus checks ignored), including the cross-process lock/crash check.
See the [bilingual installer reference](../reference/meeting-models.md#installation-behavior).

No HTTP, model download, renderer change or human speech-quality experiment. App/server stay
stopped; quality, source clocks, integration and long-session acceptance remain open, with
automatic labels still disabled.

## V5.5 checkpoint: app-owned progress after cancellation

The app's installation state now accepts progress only for its current Downloading attempt while
its cancellation token remains clear. This check shares the app-state mutex with cancellation:
a callback queued after the worker's own check cannot later overwrite counters after the owner
has moved to Cancelling or a terminal phase. The existing current-ID guard remains in place;
review, completion and final-rename result handling retain their existing authority.

The deterministic regression fails before the fix and passes afterward. It covers every
non-downloading phase, stale IDs, a valid current progress update and cancellation while the
phase still reads Downloading. All 232 app-library tests pass, including the installation
review/quota checks. The production renderer's installer check also passes consent, navigation,
stale replies, reconnection and translations with its deferred backend. There is no renderer
edit or desktop-app launch for this state-owner change. See the [bilingual installer reference](../reference/meeting-models.md#installation-behavior).

App/server stay stopped. No model download, speaker-label activation or human-quality acceptance;
physical source clocks, production analysis integration and long-session acceptance remain open.


## V5.5 consolidated reference and integration handoff

2026-10-19. This supersedes earlier next-step statements without rewriting their evidence.
The [finite implementation checklist](plan-dictation.md#v55-consolidated-delivery-checklist) owns
current status. No additional installer-only task is needed before reviewing this batch.

### Reference findings and decisions

The MIT reference credited in `NOTICE` uses its Windows Rust `speech/mod.rs` diarizer with
segmentation, bilingual advanced CAM++, automatic cluster count and threshold 0.7. Its
`notes/builder.rs` assigns model-timed words by midpoint/nearest turn, groups consecutive words
for readability and handles microphone/system sources separately. These are source observations,
not evidence that its outputs meet Vults' quality gates. Vults keeps its original reviewed model
catalog and conservative association; it does not infer a speaker from a nearby turn.

VoiceStudio's `backend/services/segmentation.py` assigns by overlap and has a silence-gap label
fallback; `diarization_local.py` resolves all dependencies locally. Its native Sortformer adapter
contains the subprocess, validates returned intervals and limits the v1 path to 120 seconds. The
feature documentation also records four-speaker and model-distribution restrictions. Only ideas
were consulted from this AGPL repository; no code was copied. Vults retains Rust/native sessions,
local verified dependencies and Unknown rather than rotating voice identities on silence.

An exact-size/SHA-256-pinned advanced bilingual embedding was compared offline before any catalog
change: 28,281,164 bytes, SHA-256
`aa3cfc16963a10586a9393f5035d6d6b57e98d358b347f80c2a30bf4f00ceba2`.
The optional spike flag `--advanced-embedding` selects only this diagnostic pair. The comparison
plan fixed sources, threshold 0.7 and decoder/window rules before inference, with no sweep.
The two already-known calibration clips gave identical DER for original and candidate:
TS3003a first 120 seconds 17.4914%, bfamdl02 first 120 seconds 17.2926%. There is no measured gain,
so the installer/catalog was not changed. Canonical calibration metadata was regenerated; raw
WAVs and human intervals are unchanged, but canonical JSON hashes are not the old fixture hashes.

### Delivered pipeline

The backend now streams up to 30 minutes through existing WAV decoding instead of collecting all
source PCM. A 10-second analysis window plus at most one 28-second decode chunk bounds retained
PCM. Embeddings remain global to the source; clustering runs once, with at most 1,080 observations.
A bounded distance matrix and heap maintain weighted average linkage and cannot-link exclusions
without repeated all-group cross-products. Deterministic tests compare its partitions with the
original direct group-mean oracle and cover the full observation budget.

The composed ASR/analysis helper rewinds the same open source descriptor. A preview-only Whisper
profile requests real token timestamps. UTF-8 token pieces become words only when valid clocks
are ordered, fit the parent segment and reconstruct its complete filtered text exactly. Punctuation
stays attached; invalid reconstruction retains the original segment. Normal ASR keeps its old
profile. Conservative association preserves Unknown for uncovered, ambiguous or overlapping
speech. Missing/corrupt/unloadable speaker assets retain ASR text/timing with an unavailable
status. It neither synthesizes word clocks nor silently selects a nearest speaker.

The actual app worker now calls this helper behind the default-off `meeting-speakers` build
feature. Native status exposes availability; an additional unchecked checkbox on each reviewed
recording freezes the choice into the job. Default builds reject that choice before consuming
review. The transcript shows anonymous Voice N/Unknown labels, source-local IDs and an explicit
failure note. View-only grouping makes nearby same-voice words readable without modifying their
stored timing. Subsecond ranges keep nonzero bounds visible. Sources are never mixed or treated
as sharing voice identities. Capture is still the existing one-minute check, not a full meeting.

Review found and resolved two concrete concurrency issues in this batch: Stage now explicitly
unlocks after cleanup so a concurrently inherited descriptor cannot prolong the lock; recording
readiness verification now checks that recording's stop flag through the shared hash loop. The
final independent read-only code review found no remaining concrete correctness issue in these
changes. This does not certify diarization accuracy.

### Native evidence and limits

Fresh temporary installation admitted six HTTP attempts, verified all ten files and ran the
pinned CPU runtime with telemetry disabled. The opt-in native tests passed cancellation during an
active operation, foreground-first retry, public-speech completion/late cancellation, the separate
VAD diagnostic and cancellation after the old three-minute boundary. Deterministic tests cover
streaming sample identity across boundaries and global recurring voices.

A frozen resource experiment analyzed the first 30 minutes of public AMI IS1000b: 61.556 seconds
wall time, peak child RSS 158,916 KiB, 420 embeddings, 6 anonymous clusters and 394 valid bounded
turns. The budget was 1,800 seconds/512 MiB. This is one Linux CPU diarization run, not ASR/capture,
a latency distribution, or an annotated quality evaluation. Temporary source audio and weights
are not repository fixtures.

Model-derived word attribution is checked separately below. Its source/reference clips are known
calibration, not held out. Restricted assigned-time agreement uses best anonymous mapping and
excludes unassigned/reference-absent time; it is not WER, semantic correctness or an acceptance
score. The initial whole-segment short-fixture coverage was zero; word timing addressed that
integration defect rather than changing the diarization threshold.

The production Meetings renderer was inspected in an isolated Tauri/WebKit profile, Black/Light
at 1180 x 820 and 480 x 360. Controlled responses exercise explicit consent/opt-in, separate public
fixture transcripts, Unknown, readable grouping, subsecond times and narrow scrolling without
horizontal overflow. Native status independently confirmed preview availability. This validates
presentation and command arguments, not actual microphone/monitor capture or physical clocks.

### Remaining release qualification

V5.5's engineering preview is delivered, while the original production quality gate stays open.
Independent disjoint human-annotated PT/EN data, all required error gates and broader hardware/
resource/capture acceptance are still needed before automatic production labels. Additional PT
corpus access requested personal registration; no form was submitted and no access gate bypassed.
A publicly available alternate archive lacked aligned human annotations and was not counted as
qualification. No calibration result is promoted to held-out evidence.

Physical cross-source synchronization and echo deletion are prerequisites for a future merged
view, not for this source-local preview. Full-duration capture, saved meetings and export belong
to V5.7/V5.8; summaries remain V5.6 and have not started. Default production identification stays
unavailable. The temporary app, isolated profile, public audio and experimental weights are cleaned
after validation; personal model files and the shared build target are preserved.


### Consolidated aggregate evidence

See [`diarization-integration-2026-10-19.json`](../../crates/spike/fixtures/diarization-integration-2026-10-19.json)
for frozen experiment plans, hashes, measured aggregates and native layout results. The final
short public speech run assigned 5/5 model-timed words; with absent assets it preserved all five
words with no labels. Known English calibration assigned 102/155 words/fallback segments;
Portuguese assigned 195/276. Assigned reference-covered time matched the best anonymous mapping
for 44730/44730 ms in English and 51563/52039 ms in Portuguese.
These restricted figures exclude Unknown and reference-absent time and do not qualify independent
accuracy. No recognized text or source audio is retained in this aggregate.

Final verification: workspace tests, 118 all-feature voice tests (7 opt-in checks ignored), 233
preview app tests, 8 evals, all-feature spike tests, workspace/feature Clippy, build, dependency
checks and repository guards pass. Five native opt-in tests were executed separately. All 37
public documentation translation hashes are current. Geometry, records, Nest close, chat models,
flock continuity, surface continuity and meeting-installer renderer checks pass. The AGENTS-listed
`scripts/check-task-actions.mjs` does not exist in this checkout and could not be run. The build's
existing large-chunk warning remains. Native layout inspection used the actual Meetings navigation selection.
