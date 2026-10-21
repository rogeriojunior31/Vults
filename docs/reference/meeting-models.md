# Meeting model artifacts

The optional N3 installer in **Dictation > Meeting settings > Meeting speaker models (experimental)**
prepares local speaker-analysis artifacts for Linux x86-64 with glibc. It does not enable speaker
labels, capture or model summaries. Ordinary Chat/dictation model downloads are unchanged.
Independent PT/EN meeting-quality and real capture acceptance remain open.

## Sources and licenses

| Component | Source and declared licenses | Included redistribution material |
|---|---|---|
| pyannote segmentation-3.0 | [Original model](https://huggingface.co/pyannote/segmentation-3.0): MIT weights; pyannote code is MIT. The original checkpoint requires a contact gate; this installer uses the public, hash-pinned ONNX re-export below without accepting that gate. | The archive's CNRS 2022 MIT notice is preserved as `SEGMENTATION-LICENSE.txt`. |
| 3D-Speaker CAM++ | Exact [weights model](https://modelscope.cn/models/iic/speech_campplus_sv_zh-cn_16k-common) and [metadata](https://modelscope.cn/api/v1/models/iic/speech_campplus_sv_zh-cn_16k-common) declare Apache License 2.0; [3D-Speaker code](https://github.com/modelscope/3D-Speaker) is Apache-2.0. The [conversion recipe](https://github.com/k2-fsa/sherpa-onnx/blob/master/scripts/3dspeaker/export-onnx.py) identifies that model. | `CAMPLUS-LICENSE.txt` retains the upstream Apache-2.0 license; `CAMPLUS-NOTICE.txt` records source and conversion provenance. Vults does not modify the graph or link the publishing engine. |
| ONNX Runtime 1.31.0 CPU | [Official Microsoft release](https://github.com/microsoft/onnxruntime/releases/tag/v1.31.0), MIT. There are no model weights in the runtime. | The exact archive's license, full third-party notices and privacy document are preserved separately. Header files, symlinks and unused provider libraries are not installed. |

The backend uses `ort` under MIT OR Apache-2.0 and `knf-rs` under MIT with Apache-2.0 native
filterbanks. The optional installer uses `tar` (MIT OR Apache-2.0), `flate2` (MIT OR Apache-2.0)
and `bzip2` (MIT OR Apache-2.0). Its pure Rust `libbz2-rs-sys` dependency has the
[permissive bzip2 license](https://github.com/trifectatechfoundation/libbzip2-rs/blob/v0.2.5/COPYING).
The unmodified `BZIP2-LICENSE.txt` is included; Cargo's reviewed exception is scoped to version
0.2.5. No GPL-linked speech engine is added.

## Download pins

These are fixed artifacts, not a request for the latest release. The installed graph hashes match
the V5.4 experiment. The installer follows only approved GitHub HTTPS release/CDN redirects.

| Artifact | Download size (bytes) | SHA-256 |
|---|---:|---|
| [Segmentation archive](https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2) | 6,958,444 | `24615ee884c897d9d2ba09bb4d30da6bb1b15e685065962db5b02e76e4996488` |
| [CAM++ graph](https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx) | 28,281,138 | `f682b514c05d947ee3fa91cd6ec6c5c7543479a128373fa29b1faedccd21fd11` |
| [ONNX Runtime Linux x64 archive](https://github.com/microsoft/onnxruntime/releases/download/v1.31.0/onnxruntime-linux-x64-1.31.0.tgz) | 11,408,003 | `cc5c72baf5ae5c8238a6841f0897227be2d02826b9cf98eaf02fdefaeeb03a57` |

The three downloads total **46,647,585 bytes**, excluding HTTP overhead. The expanded segmentation
graph is 5,992,913 bytes, SHA-256 `220ad67ca923bef2fa91f2390c786097bf305bceb5e261d4af67b38e938e1079`.
The expanded runtime library is 29,345,384 bytes, SHA-256
`469849977cb60744ea160927e1a3a0643fd4c55c0aff743251c6d293402e334e`.
The [compiled catalog](../../crates/voice/src/meeting/assets/catalog.rs) also pins the extracted
license/notice/privacy files. The complete bundle has ten files; supplied notices participate in
its review fingerprint too.

## Installation behavior

Installation needs a matching reviewed catalog fingerprint and a caller-supplied gate for every
HTTP attempt, including redirects. In the app, expand the experimental section, choose
**Review installation**, read the sources/licenses and check the consent box, then choose
**Download and install** within one minute. Review reads and verifies local files without HTTP.
Only the Settings webview can review, install, cancel or read the installer state; Meetings links
to that configuration owner. The one-use review is bound to the manifest and local request ledger.

The app gate durably reserves each HTTP attempt before sending, with a limit of 24 per rolling
24 hours, including redirects, failures and interrupted attempts. Restarting or clearing history
does not reset it; a missing, replaced or unreadable ledger refuses the reviewed attempt. Local
traces contain only artifact IDs, outcomes and timing, never audio, transcripts or URLs.
Installation continues across navigation; cancel in this section or quit the app to stop it.
Leaving an unconfirmed review discards it. There is no automatic retry, resume, update check or
proxy use. A failed attempt requires a fresh review. The separate developer command retains its
twelve-request invocation cap and does not replace the app gate.

Progress updates are accepted only for the current active download. After cancellation or
completion, late updates are ignored; a new review refreshes the installation state.

A private staging directory and an exclusive lock isolate the attempt.
The lock must be a regular file owned by this user, mode 0600 and with one hard link. Invalid
locks are refused before abandoned staging data is removed; they are not replaced or repaired.

Streaming downloads and selected archive members must match size and SHA-256. Each artifact has a shared 180-second
network/body deadline across redirects, checked between chunks and at end of stream even when
the next result is immediately ready. An expired attempt cannot reserve another HTTP request.
Cancellation and expiry are rechecked after quota admission and response receipt; an admitted
attempt remains charged and its trace closes as cancelled or failed. Local file flush/sync and
extraction are separate work.
Pre-cancelled downloads stop before client or temporary-file setup. Cancellation is rechecked
after receiving a chunk, before reporting write progress and after file flush/sync. A chunk
returned with cancellation already set is discarded without writing or progress. Writes already
underway can finish before cancellation is observed.

Extraction rejects unsafe paths, duplicate
selected members, links in place of selected files and expansion/item-budget violations. Nothing
from an archive is executed. Only a complete verified bundle is published by atomic rename.
Member files and their staging directory are synchronized before publication, followed by the
installation root after rename. Cancellation is checked again after the staging-directory sync.
If that sync fails, publication is skipped and staging cleanup is attempted. File synchronization
alone does not guarantee persistent directory entries; see [Linux fsync](https://man7.org/linux/man-pages/man2/fsync.2.html).

Readiness rechecks every file, including notices. Cancellation or failure attempts to remove staging data;
a cancellation arriving after the final rename can find an already-completed installation.
Pre-cancelled readiness checks stop before bundle lookup or file opening. Hash verification checks
cancellation after each read, including EOF, so a cancelled check cannot report verified bytes.
Extraction checks cancellation before creating output, after member reads and file synchronization,
and after the archive's final marker. Blocking filesystem work must return before cancellation can
be observed; a cancelled extraction never makes its partial staging files ready.

Version directories include the review fingerprint, preserving previously installed versions and
loaded runtimes. An existing corrupt/incomplete version is reported and preserved, not silently
repaired or replaced. Abandoned staging attempts are removed under the installation lock on the
next explicit attempt. Recovery checks cancellation between abandoned entries. A recursive removal
already started can finish; remaining entries wait for the next explicit attempt.
Native session shutdown releases its contexts, while a runtime library
stays pinned for that process; changing its version requires a new process.

## Runtime privacy

The process sets `ORT_DISABLE_TELEMETRY=1` before creating workers. Native loading requires that
setting and also disables the API telemetry option. [Upstream privacy documentation](https://github.com/microsoft/onnxruntime/blob/v1.31.0/docs/Privacy.md#disabling-telemetry)
explains why API configuration alone is insufficient. These are process settings, not edits to
the user's shell configuration.

## Validation status

The developer path has been checked with fresh public downloads, complete installation, cached
revalidation without HTTP, invalid/cancelled streams and archive failures. The installed 1.31.0
runtime passes the bounded public-speech, cancellation and foreground-first retry checks with
startup privacy enforced. This evidence does not qualify full meetings or automatically enable
speaker attribution in the app.


A separate-process regression holds the real installation lock in a child and verifies that a
competing attempt preserves its active staging files. After SIGKILL bypasses destructors, the
next attempt reacquires the lock, removes the abandoned stage and preserves a previous-version
marker. This check uses private synthetic fixtures; power-loss and storage-fault acceptance remain
separate.

The app's separate checks cover review/consent, Settings-only authority, persistent quota rejection
before HTTP and cancellation without a download. Native Black/Light layouts at 1180 x 820 and
480 x 360 are inspected; progress/verified-state views use controlled responses without HTTP.
Installation alone keeps speaker analysis disabled. Normal builds do not expose identification.
A developer build with `--features meeting-speakers` offers an additional unchecked option for each
reviewed recording: **Also identify voices for this recording (experimental)**. That preview
runs only installed local artifacts, preserves plain transcription if analysis is unavailable and
keeps uncertain speech as **Unknown voice**. The choice resets for every recording. Microphone and
system audio retain separate clocks and voice IDs; they are not synchronized or merged.

The native backend can stream up to 30 minutes per source, with one global clustering pass. A
public 30-minute CPU diarization run took 61.556 seconds and peaked at 158,916 KiB RSS. This is one
resource experiment, not full transcription/capture or independent quality qualification. The app
now supports four-hour manual recording with bounded PCM16 storage, but speaker analysis remains
limited to 30 minutes per source; longer recordings keep plain transcription without labels. Native
cancellation/preemption checks pass,
including cancellation beyond three minutes; blocking IO and native construction remain
cooperative. The source-local preview has also been inspected in isolated native Black/Light
windows using controlled responses. These checks do not pass the independent PT/EN quality gates
or enable automatic production labels.


The offline [speaker evaluation reference](meeting-evaluation.md) describes per-source clock
checks and error measurements for supplied annotations. Its synthetic examples do not qualify
real PT/EN speech or activate production analysis.
