# Meeting speaker evaluation

This offline developer tool measures anonymous speaker turns against supplied annotations. It
never enables speaker labels in Vults. It does not record, read audio, load models, download
anything or call a provider. The optional native spike can attach the same report to a local
analysis; real PT/EN meeting acceptance remains open.

## Run

```sh
cargo run -p vults-spike --bin vults-meeting-eval -- reference.json prediction.json
```

Both files use schema 1 and the same `clock`: anonymous `recording` and `source` identifiers,
lowercase `wavSha256` and `durationMs`. Equal duration without matching identity/hash is refused.
The standalone tool compares these declared clocks; it cannot verify audio it never opens.

The reference also declares `language` (`en` or `pt`), `annotation` (`human`, `controlled` or
`synthetic`) and `split` (`calibration` or `evaluation`). These are provenance declarations, not
independent verification of consent, annotation quality or a disjoint corpus split. Each turn has
`startMs`, `endMs` and an anonymous integer `speaker`. Prediction `speaker: null` means unassigned
activity; reference turns must have an identity. No recognized words, personal names or audio
paths belong in these files. See the [synthetic examples](../../crates/spike/fixtures/meeting-eval/).

Times are half-open milliseconds on one source's own clock. Never concatenate microphone and
system annotations or infer common identities from equal numeric labels. Files are limited to
one MiB, 4,096 turns per side, eight known speakers per side and 180 seconds per source, matching
the current native worker. Unknown fields, invalid bounds and unsupported schema are rejected.
Linux rejects symlinks and non-regular inputs without blocking on a FIFO.

## Scoring profile

The profile is fixed: the entire source is evaluated, with **zero collar** and overlap included.
It follows the optimal one-to-one label matching and error decomposition described by
[pyannote.metrics](https://pyannote.github.io/pyannote-metrics/_modules/pyannote/metrics/diarization.html),
using an independently implemented exact subset solver. Missed, false-alarm and confused speaker
time are reported separately; DER is their sum divided by reference speaker time. Speaker time
counts overlapping reference voices separately. Duplicate activity for one label counts once.
Rates are ratios: `0.4` means 40%; false alarms can make the result exceed 100%.

`falseAlarmSpeakerMs` is also partitioned into `falseAlarmWithoutReferenceSpeakerMs` (no
reference voice active) and `excessOverlapSpeakerMs` (more predicted voices than active reference
voices while speech is annotated). The two sum to the total, including unknown prediction
activity. This distinguishes speech-detection errors from excess overlap without changing DER
or label matching. Reference absence is not proof of acoustic silence: annotations can omit
laughter or other vocal sounds.

Unassigned prediction activity is an explicit extension: it is one unmappable activity stream,
never a candidate identity. During speech it is penalized as confusion or missing overlap;
during silence it is a false alarm. Multiple overlapping null turns count as one unknown stream;
they cannot establish how many unknown people spoke. `unknownActivityMs` reports its union.
A silence-only reference has `der: null`, while false-alarm time remains visible.

The report retains annotation/language/split metadata and SHA-256 fingerprints of the canonical
schema-1 serialization of each input, not their whitespace. It contains no acceptance threshold
and always reports `productionQualified: false`. A declared human reference or a perfect
synthetic score does not qualify production. Freeze a reviewed, consented human PT/EN corpus,
disjoint splits and acceptance limits before tuning the model. Echo, noise, short/quiet turns,
overlap and long-session resource/clock acceptance remain separate requirements.

## Native spike

Build the optional offline analysis executable and provide reviewed local artifacts:

```sh
cargo build -p vults-spike --features diarization --bin vults-diarization-spike
target/debug/vults-diarization-spike \
  --runtime "$DIARIZATION_RUNTIME" --runtime-sha256 "$DIARIZATION_RUNTIME_SHA256" \
  --segmentation "$DIARIZATION_SEGMENTATION" --embedding "$DIARIZATION_EMBEDDING" \
  --wav "$DIARIZATION_FIXTURE" --reference reference.json
```

With a custom `CARGO_TARGET_DIR`, use its `debug/` executable. The actual WAV
fingerprint must match the annotation before native initialization; normalized duration must
also match before a report is published. The resulting `quality` field uses the same scorer.
The WAV is hashed and analyzed through the same open descriptor, then hashed again; changed
content discards the result. This does not create an immutable audio snapshot. Without the
option, ordinary spike output retains `quality: null`. No source identity or
synchronization across devices is inferred by this helper.

The bundled EN/PT examples contain invented timelines, language tags and a placeholder hash;
no spoken language, recording or human annotation was measured. The English-tagged case checks
permuted labels; the Portuguese-tagged case has equal speaker counts but 40% confusion. Their
scores are scorer regressions, not evidence of English or Portuguese model accuracy.

## Human calibration on 2026-10-19

Two preselected 120-second public clips were measured with the fixed 0.7 clustering threshold,
zero collar and overlap retained. The manifest, clip/reference hashes and provisional DER target
of 0.20 were frozen before inference. These are calibration results; no threshold was tuned and
there is no held-out or production acceptance claim.

| Source | Annotated / found voices | DER | Native total | Peak RSS |
| --- | ---: | ---: | ---: | ---: |
| AMI IS1000a, first 120 s | 3 / 3 | 82.12% | 3,859 ms | 130,344 KiB |
| C-ORAL-BRASIL bfamdl02, first 120 s | 2 / 1 | 39.79% | 8,950 ms | 155,668 KiB |

The English input is from the [AMI corpus](https://groups.inf.ed.ac.uk/ami/corpus/) (CC BY 4.0).
Its word-derived references come from the [BUT Speech@FIT setup](https://github.com/BUTSpeechFIT/AMI-diarization-setup/tree/2509d8933721023fab4def2618aabd5c28eb82e9),
using manual AMI 1.6.2 annotations. Attribution: J. Carletta et al., *The AMI meeting corpus:
A pre-announcement* (2006); F. Landini et al., *VBx*. This recording belongs to the published
training partition; independence from model training has not been established.

The Brazilian Portuguese input is the [public C-ORAL-BRASIL sample](https://www.c-oral-brasil.org/english-site/c-oral-brasil-i_example.php),
with manual WinPitch `UNIT` alignment. The [corpus license](https://www.c-oral-brasil.org/c-oral-brasil-i.php)
is CC BY-NC-SA 4.0; this sample was used locally for noncommercial research. Attribution:
Tommaso Raso and Heliana Mello (eds.), *C-ORAL-BRASIL I*, Editora UFMG (2012).
No source audio, transcripts or annotation turns are redistributed here.

The Portuguese reference uses utterance intervals, including their pauses; the English setup
uses words. The percentages therefore are not directly comparable across corpora. The English
clip has only 28,610 reference speaker-ms in its first 120 seconds, so its false alarms matter
substantially. The Portuguese reference has 133,901 speaker-ms because overlap counts each
voice; its two voices collapse into one in the baseline. Neither result meets the provisional
diagnostic target. Automatic speaker labels remain unavailable.

The [content-free evidence](../../crates/spike/fixtures/meeting-human-calibration-2026-10-19.json)
retains the frozen manifest, source/license attribution, model pins, timing and complete error
components. Independent one-millisecond raster/exhaustive-label scoring agrees with the native
and standalone Rust reports. CPU sessions used one intra/inter-op thread on a Ryzen 7 5700X3D;
RSS is Linux `wait4` for the whole child. One run per clip is not a latency distribution.
`totalMs` includes load, inference and the final source hash; whole-process wall time also
includes initial hashing/model admission.

To reproduce, fetch the exact public sources/revisions recorded in the evidence. Extract only
`bfamdl02.wav` and `bfamdl02.xml` from the linked sample. Retain the first `rate * 120` PCM frames
without changing rate, channels or amplitude, writing a canonical 44-byte PCM16 WAV. AMI stays
16 kHz mono; C-ORAL stays 22.05 kHz stereo and is normalized by the native worker. For AMI, use
only `SPEAKER IS1000a 1` rows from `only_words/rttms/train/IS1000a.rttm`; derive the end from start
plus duration before rounding. For C-ORAL, use `speaker`, `startTime` and `endTime` from `UNIT`,
without retaining text or personal headers. Round endpoints to the nearest millisecond, ties up;
clip to `[0,120000)`, discard empty intersections and assign anonymous integer labels by sorting
the full source's speaker codes. Set `annotation: human`, `split: calibration` and the exact clock
from the frozen manifest, then use the native command above. Check reference fingerprints before
comparing results. The source downloads, annotations and disposable model/runtime bundle
were removed after validation; the user's audio/profile/cache was not used.

## Coactivity correction on 2026-10-19

The development candidate prevents two embedded local observations active together in an owned
segmentation frame from receiving the same global label. A forbidden pair also stays separate
after intermediate group merges. Unknown observations do not create constraints. The models,
embedding policy and cosine distance threshold of 0.7 are unchanged; spike output now declares
`coactivityConstraints: true`.

The exact original WAV/reference fingerprints were reproduced, and both clips were measured
once again with zero collar and overlap retained. These remain calibration clips inspected
during development, not a held-out evaluation set.

| Source | Found voices, before / after | DER, before / after | Native total | Peak RSS |
| --- | ---: | ---: | ---: | ---: |
| AMI IS1000a, first 120 s | 3 / 3 | 82.12% / 82.12% | 3,847 ms | 130,324 KiB |
| C-ORAL-BRASIL bfamdl02, first 120 s | 1 / 2 | 39.79% / 17.29% | 9,029 ms | 148,248 KiB |

The English prediction fingerprint is identical. In Portuguese, confused speaker-ms fall from
26,015 to 1,428 and missed speaker-ms from 26,617 to 17,880, while false alarms rise from 641 to
3,847. The Portuguese candidate meets the original provisional diagnostic target of 0.20; the
English candidate does not. This is no production qualification: predicted coactivity can itself
be wrong, and a correction developed after inspecting these clips needs separate evaluation.

The [comparison evidence](../../crates/spike/fixtures/diarization-coactivity-2026-10-19.json)
preserves the original evidence hash and aggregate candidate reports. Native, standalone and
independent millisecond raster/exhaustive mapping scores agree. A deterministic regression
checks direct and intermediate forbidden merges. Native completion, late cancellation, active
termination and interactive preemption checks pass. Temporary public audio, annotations and
model/runtime files were removed afterward; no user recording or profile was used.

The next quality step needs recording/speaker-disjoint human evaluation material with acceptance
limits frozen before further tuning. Physical microphone/system alignment, echo, quiet turns,
noise and long-session resources remain open. Automatic speaker labels remain unavailable.

## Pause and diagnostic refinements on 2026-10-19

The development worker now merges only touching or overlapping same-label turns. The former
17 ms tolerance could fill a complete missing segmentation frame and incorrectly make a text
segment appear continuously covered by one voice. The regression checks 1, 16, 17 and 1,000 ms
gaps, known and unknown observations, adjacent turns and downstream text association. Short or
quiet turns are not filtered out. Spike output declares `fillsSpeechGaps: false`; models,
embeddings, coactivity constraints and the 0.7 distance threshold are unchanged.

Two further English sources were selected before their respective experiments from the pinned
[BUT AMI test meeting list](https://github.com/BUTSpeechFIT/AMI-diarization-setup/blob/2509d8933721023fab4def2618aabd5c28eb82e9/lists/test.meetings.txt).
IS1009a was evaluated for the pause correction, then explicitly moved to calibration when its
errors motivated another grouping experiment. ES2004a was selected before that second candidate.
Both use the first 120 seconds, the same AMI public mirror/license/attribution and word-derived
`only_words/rttms/test/` references. They are distinct recordings, but speaker identity and
model-training independence are not established. No independent Portuguese source was added.

| Retained worker, first 120 s | Annotated / found voices | DER |
| --- | ---: | ---: |
| IS1000a, original English calibration | 3 / 3 | 82.12% |
| bfamdl02, original Portuguese calibration | 2 / 2 | 17.29% |
| IS1009a, expanded English calibration | 4 / 2 | 61.09% |
| ES2004a, further English evaluation | 3 / 1 | 31.33% |

The original calibration predictions are unchanged. Preserving pauses adds 17 missed reference
speaker-ms in IS1009a (DER 61.06% to 61.09%), so the frozen no-regression quality gate **fails**.
The change is retained for timestamp/continuous-coverage correctness in the development worker;
it is not a quality improvement or release acceptance. A stronger rule forbidding merges of
all local slots, including non-overlapping ones, was tested and removed: it worsened IS1000a
from 82.12% to 90.64%, despite improving individual other clips. No threshold sweep was used.

The new false-alarm partition identifies different problems. IS1000a has 15,329 speaker-ms with
no reference voice active and zero excess overlap. Portuguese has 649 and 3,198, respectively.
Those are annotation-relative findings, not verified acoustic causes. Listening/annotation
review and independent speech/overlap evaluation are needed before changing detection rules.

Six controlled calibration variants exercise quieter audio, uniform noise and delayed echo:

| Transformation | English DER | Portuguese DER |
| --- | ---: | ---: |
| Gain 0.1 | 82.00% | 17.23% |
| Uniform noise, 15 dB whole-source SNR | 90.10% | 17.67% |
| Same-source echo, 200 ms delay, gain 0.35 | 84.83% | 31.62% |

These retain the original sample rate, average stereo channels to mono, round/clamp PCM16 and
use propagated anonymous intervals marked `annotation: controlled`, `split: calibration`.
Echo references add the delayed same-speaker intervals, clipped to the source duration; duplicate
speaker activity still counts once. Noise uses the deterministic generator/seed in the evidence.
They are not new human annotations, a language ranking or physical microphone/system validation.
The Portuguese noise control also adds 17 missed speaker-ms when artificial gap filling is removed.

The [refinement evidence](../../crates/spike/fixtures/diarization-refinements-2026-10-19.json)
records frozen experiment plans, exact source/reference hashes, the rejected candidate, retained
reports and controlled transformations. Native and standalone reports agree with independent
millisecond raster/exhaustive mapping, including both new false-alarm counters. The former gap
filling was reconstructed and checked against three separate native baseline invocations.
The voice suite passes 90 tests, the spike suite 22 and the explicit native checks 3.
No source audio, text or annotation turns are redistributed; disposable downloads/models/runtime
are removed afterward and the app remains closed. Automatic speaker labels stay unavailable.

## Decoder ties and reference audit on 2026-10-19

The development decoder now keeps the first exact maximum powerset class, following the
[original hard decoder](https://github.com/pyannote/pyannote-audio/blob/3.0.0/pyannote/audio/utils/powerset.py)
and [first-maximum argmax semantics](https://docs.pytorch.org/docs/2.14/generated/torch.argmax.html).
The former last-maximum selection chose the final overlap class when all seven scores tied.
The regression reproduces that failure and covers every maximum pair, unique maxima, signed
zeros and one-float-step near ties. No confidence tolerance or speech/overlap threshold was added.
Native output declares `argmaxTieBreak: first`.

All four primary prediction fingerprints and complete quality reports match the preceding
refinement evidence. This is decoder-contract correctness, not a measured accuracy gain on
these clips. Model pins, embeddings, coactivity constraints, pause preservation and the 0.7
clustering threshold are unchanged.

The same predictions were also scored against the pinned BUT AMI
`word_and_vocalsounds/rttms/` references. The
[reference authors](https://github.com/BUTSpeechFIT/AMI-diarization-setup/blob/2509d8933721023fab4def2618aabd5c28eb82e9/README.md)
publish these as an alternative but prefer word-only scoring because vocal-sound annotation is
inconsistent. The alternate references are diagnostic calibration material; the primary profile
remains word-only with zero collar and overlap included.

| First 120 s | Word-only false-alarm speaker-ms | With-vocals false-alarm speaker-ms | Covered by broader reference |
| --- | ---: | ---: | ---: |
| IS1000a | 15,329 | 15,329 | 0 |
| IS1009a | 10,445 | 10,445 | 0 |
| ES2004a | 3,244 | 3,121 | 123 |

The broader annotations do not explain most false alarms on these crops. They also change
reference speaker time and optimal mapping, so a different DER is not a model improvement:
IS1000a becomes 83.22%, IS1009a stays 61.09% and ES2004a becomes 35.41%. These comparisons do
not establish actual acoustic causes without listening/annotation review. Portuguese retains
its original report; no additional Portuguese annotation or recording was introduced.

The [reference-audit evidence](../../crates/spike/fixtures/diarization-reference-audit-2026-10-19.json)
retains the pre-inference plan, original evidence hash, seven reference fingerprints, download pins,
aggregate reports and source/license attribution. Reproduce with the same crop preparation above,
using the pinned `word_and_vocalsounds/rttms/train/IS1000a.rttm` and
`word_and_vocalsounds/rttms/test/{IS1009a,ES2004a}.rttm`. Map anonymous labels from the complete
annotation and keep the exact primary source clock. Score the native prediction unchanged against
each reference. Do not replace the primary result with the diagnostic result.

Seven standalone reports agree with independent millisecond raster/exhaustive-label scoring;
all four primary native reports are unchanged. Validation passes 91 voice tests, 22 spike tests
and 3 explicit native checks. Temporary audio/annotations/models/runtime are removed afterward;
the app remains closed and automatic speaker labels stay unavailable. The next detection
experiment can assess the existing Silero speech detector independently, with a frozen profile
and attention to missed short/quiet speech; this audit does not enable that filter.

## Existing speech-detector evaluation on 2026-10-19

The offline spike can compare its unchanged diarization with a speech-mask candidate using the
existing Silero detector. Append `--vad` after a required reference:

```sh
target/debug/vults-diarization-spike \
  --runtime "$DIARIZATION_RUNTIME" --runtime-sha256 "$DIARIZATION_RUNTIME_SHA256" \
  --segmentation "$DIARIZATION_SEGMENTATION" --embedding "$DIARIZATION_EMBEDDING" \
  --wav "$DIARIZATION_FIXTURE" --reference reference.json --vad "$DIARIZATION_VAD"
```

Supply an absolute path to a trusted, immutable copy of the existing catalog's
`ggml-silero-v6.2.0.bin`: 885,098 bytes, SHA-256
`2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987`.
The [ggml conversion](https://huggingface.co/ggml-org/whisper-vad) and
[upstream detector](https://github.com/snakers4/silero-vad) declare MIT licensing.
The spike verifies the size/hash before native initialization; Linux loads through the retained
descriptor, then the model and source are rehashed before publication. This detects persistent
edits; it does not create an immutable snapshot or rule out transient edits. No model is downloaded
by this command.

Without the option, `vadDiagnostic` is null and the original analysis runs alone. With it, the
top-level `quality`/`turns` remain the unchanged baseline and `vadDiagnostic` contains the candidate
quality, speech regions and intersected turns. `totalMs` remains baseline timing; diagnostic
timing is separate. Whole-child RSS covers both engines and does not isolate VAD overhead.
The diagnostic uses the same source descriptor/normalization and three-minute limit. Its native
load, inference and destruction share one scheduler slot, admitted behind interactive work.
Silero cannot abort inside that call; cancellation discards the completed result. This is an
offline experiment, not a production preemption implementation or an app speech filter.

The profile was frozen before measurement: existing smoothing/thresholds, 250 ms speech and
400 ms silence confirmation, one CPU thread and the existing 200 ms margin on each speech region.
Intersecting those regions only removes baseline activity; it preserves timestamps, known/unknown
IDs and overlap. It never concatenates audio or fills baseline gaps. An executable regression
checks those properties, one-millisecond turns, duration mismatch and the 4,096-turn budget.

| Human source, first 120 s | Baseline DER | Masked DER | Additional missed speaker-ms |
| --- | ---: | ---: | ---: |
| IS1000a, English | 82.12% | 50.73% | 5,235 |
| bfamdl02, Portuguese | 17.29% | 17.57% | 566 |
| IS1009a, English | 61.09% | 57.48% | 1,026 |
| ES2004a, English | 31.33% | 44.16% | 10,685 |
| TS3003a, new English recording | 17.49% | 31.26% | 12,875 |

TS3003a was preselected from the pinned BUT test list before candidate output, using the same
public AMI mirror, first-120-second crop and word-only annotation recipe. It is a different
recording, not established speaker/model-training independence. The four known human clips
and six previously defined controlled quiet/noise/echo transforms remain calibration material
for this assessment; the old ES2004a evaluation declaration is retained as historical metadata.
No independent Portuguese human source was added.

The mask cuts IS1000a false alarms from 15,329 to 1,368 speaker-ms but loses one previously
covered reference turn. ES2004a and TS3003a each lose four. Quiet controls add 12,370 missed
speaker-ms in English and 3,253 in Portuguese; noise adds 9,952/911 and echo adds 8,385/885.
A controlled 200 ms public-domain speech fragment loses all its 170 ms of baseline coverage.
That fragment is not a human-annotated complete word. Its DER drops from 133% to 100% as false
alarms/confusion disappear, illustrating why lower aggregate DER alone cannot qualify a filter.

**Reject general speech masking.** All 12 cases fail the frozen no-additional-missed-speech gate;
several also fail DER and newly-uncovered-turn checks. The optional diagnostic stays available
for reproduction. The app and default diarization are unchanged. This experiment does not tune
Chat/dictation thresholds or justify removing their existing VAD behavior.

Missed time is relative to the supplied annotations. C-ORAL utterance intervals include pauses,
so its additional missed time alone is not proof of lost phonetic content. Acoustic validation
remains open; the English word-coverage regressions and controlled short-fragment loss already
prevent accepting this profile.

The [VAD evidence](../../crates/spike/fixtures/diarization-vad-evaluation-2026-10-19.json) retains
the frozen plan, model/source/reference hashes, controlled recipes, all outcomes and aggregate
resources. All ten known baseline reports reproduce prior fingerprints/components; 24 reports
agree with standalone scoring and an independent millisecond raster/exhaustive mapping oracle.
Each masked interval is verified as a same-identity subinterval of a baseline turn. Validation
passes 91 feature voice tests, 23 spike tests, 4 explicit native checks and 57 default voice
tests with the real Silero fixture checks enabled. Three invalid VAD invocations reject before
native runtime admission; a no-option native run confirms `vadDiagnostic: null`.
Temporary source audio, annotations, controls and models/runtime are removed afterward. The app
remains closed, and automatic speaker labels stay unavailable. Further meeting-specific detection
work needs separate acceptance for short/quiet speech and production cancellation; the shared
dictation profile must not be retuned from this failed candidate.

## Meeting-frame speech profile on 2026-10-19

A second, separately frozen offline candidate removes the dictation detector's smoothing and
250 ms speech confirmation. `--vad-frames` replaces `--vad` in the command above; both require
a reference and the same catalog-pinned immutable model. Its explicit `meeting-frames` profile
keeps each raw 512-sample frame with probability at least 0.5, joins adjacent selected frames,
and applies the unchanged 200 ms margin/merge. At 16 kHz, frames are 32 ms; the final partial
frame stays within the source. Inputs shorter than one frame still return no activity.
There is no threshold, dwell or margin sweep, and no new model or dependency.

The optional API requires a `SpeechProfile`: `Dictation` reproduces the earlier diagnostic;
`MeetingFrames` selects this candidate. Chat/dictation keep their original detector and timings.
The CLI records the selected profile in `vadDiagnostic.profile`. Baseline analysis, source/model
admission, clock/identity preservation and the native cancellation limitation described above
remain the same. Neither profile is an app filter or an automatic speaker-label feature.

The previous 12 cases are all known calibration for this new candidate, including TS3003a and
the former evaluation metadata. EN2002a's first 120 seconds were preselected from the same pinned
BUT test list before candidate output as one additional English recording. Its word-only
reference uses the unchanged crop, rounding and anonymous-label recipe. This establishes a new
recording for this experiment, not speaker/model-training independence; an independent Portuguese
human evaluation is still missing. The frozen acceptance again requires no increase in false
alarms, DER or missed speaker-ms, and no previously covered reference turn becoming uncovered.

| Case | Baseline DER | Frame-mask DER | Additional missed speaker-ms | Newly uncovered turns |
| --- | ---: | ---: | ---: | ---: |
| IS1000a, English | 82.12% | 52.74% | 4,434 | 1 |
| IS1009a, English | 61.09% | 59.18% | 306 | 0 |
| ES2004a, English | 31.33% | 40.72% | 8,397 | 3 |
| TS3003a, English | 17.49% | 28.45% | 10,051 | 0 |
| EN2002a, new English recording | 35.90% | 37.26% | 2,987 | 0 |
| bfamdl02, Portuguese | 17.29% | 17.53% | 474 | 0 |
| English quiet control | 82.00% | 62.37% | 10,536 | 1 |
| English noise control | 90.10% | 64.78% | 8,221 | 1 |
| English echo control | 84.83% | 70.56% | 7,163 | 4 |
| Portuguese quiet control | 17.23% | 20.00% | 3,885 | 0 |
| Portuguese noise control | 17.67% | 17.91% | 774 | 1 |
| Portuguese echo control | 31.62% | 31.62% | 205 | 0 |
| Controlled 200 ms fragment | 133.00% | 133.00% | 0 | 0 |

The short control retains all 170 ms of its baseline coverage, compared with zero under the
previous profile. Its unchanged 133% DER still exposes the baseline's false alarms and unmapped
identity; preserving a controlled fragment does not qualify short-word recognition. The new
profile reduces additional misses on the four known English human clips compared with the
previous diagnostic, but does not remove them. It is not uniformly better: the Portuguese quiet
control adds 3,885 missed speaker-ms, compared with 3,253 under the previous profile.

**Reject general hard masking again.** All six human clips and six stress controls fail the
no-additional-missed-speech gate; only the controlled short fragment passes. EN2002a adds 2,987
missed speaker-ms despite lower false alarms. The C-ORAL utterance/pause caveat remains relevant;
English word-coverage regressions independently prevent acceptance. Neither a lower average DER
nor the short-control improvement overrides these failures. No follow-up parameter sweep is run.

The [meeting-frame evidence](../../crates/spike/fixtures/diarization-meeting-frames-2026-10-19.json)
retains the frozen plan and its hash encoding, source/reference/model/code hashes, exact prior
controlled recipes, aggregate reports, resource measurements and licenses. All 12 known baseline
reports are unchanged; 26 native reports agree with standalone scoring and an independent
millisecond raster/exhaustive mapping oracle. Every mask output remains a same-identity subset
of the baseline, with removed activity accounted for by reduced false alarms plus additional
misses. The old `--vad` profile reproduces its earlier English/short-control reports, and an
ordinary run retains null diagnostic output. Four invalid candidate admissions fail before
native runtime loading.

Validation passes 92 feature voice tests, 23 spike tests, 4 explicit native checks and 57 default
voice tests with real Silero fixtures, plus all-feature Clippy, formatting and repository guards.
The pure regression distinguishes a short burst from dictation's dwell behavior and covers exact
thresholds, silence, gaps and partial source edges; native admission/cancellation checks exercise
both profiles. Temporary audio, annotations, controls and models/runtime are removed afterward;
the app stays closed and existing user/shared caches are preserved. Default diarization and
Chat/dictation behavior are unchanged. Next work should address detection/overlap errors with
independent speech-preservation evidence rather than continuing hard-mask parameter sweeps;
physical source clocks, independent Portuguese evaluation, production cancellation and
long-session acceptance remain open.

## Millisecond timeline edges

Sample-level frame clipping can produce an interval whose start and end both convert to the
same millisecond. Speaker analysis now drops that zero-duration interval before merging turns;
it does not extend the interval to make it fit. Previously, an isolated collapsed edge could
make text association or offline scoring reject the entire result. Positive-duration turns,
including a one-millisecond turn, retain their labels and bounds; gaps stay gaps.

The shared WAV reader requires at least one millisecond on the normalized source clock, before
allocating the resampler or reporting progress. This single check covers transcription, native
speaker analysis and both optional speech diagnostics. Previously transcription could publish
an empty zero-duration result while only speaker analysis enforced the minimum. Valid sources with a duration of one millisecond remain accepted. This is clock representability, not speech recognition.

Failing-before/passing-after regressions cover clipped known/unknown turns with text association,
transcription admission before progress/decode, and source boundaries across seven sample rates
(including 8,001/22,050/44,100 Hz), mono/stereo and float32/PCM16/24/32. Fixtures for chunk coverage
and non-finite samples use the smallest valid duration so those checks retain their purpose.
Validation passes 98 feature voice tests (6 model-dependent checks ignored), 59 default voice
(2 ignored), 23 spike tests, all-feature Clippy, formatting and repository guards. This refinement
uses synthetic WAVs without native inference, model/audio downloads or app activation; human
speech-quality and real-meeting acceptance remain open.

## Native operation preparation

Before each native segmentation/embedding call, the controller rechecks cancellation and pending
interactive speech after constructing the per-call options. A cancellation observed at that
boundary returns no result without starting inference. An already queued interactive request
releases the prepared options and background slot; the operation prepares fresh options after
interactive work finishes. Previously these requests could still start unnecessary native work
before the interruption watcher reacted. Requests arriving during inference retain the existing
abort/discard/retry path. Session-construction and Silero whole-call cancellation limits above
still apply.

Two deterministic regressions fail before the fix and pass afterward, covering cancellation and
foreground arrival during option creation. Checks pass 96 feature voice tests (5 ignored),
23 spike tests and 3 explicit native checks for pipeline completion/late cancellation, active
termination and foreground-first retry, plus all-feature Clippy, formatting and repository guards.
Native checks use the existing pinned model/runtime bundle, tracked speech fixture and synthetic
input. Temporary models/runtime and logs are removed afterward; no personal recording/profile or
app activation is used. This validates operation scheduling, not human speech quality; automatic
speaker labels and the real-meeting acceptance gates remain open.

## Shared speech-detector ownership

Silero construction, interactive detection and context destruction now all use the shared native
work scheduler. This covers Chat's end-of-speech watcher, final speech trimming and dictation's
speech-presence retry check. Previously construction was serialized, while detection/freeing
could bypass other native speech work. Interactive detection now enters the same priority queue
as Whisper. No detector thresholds, hysteresis, padding or source timestamps are retuned.

The offline speech diagnostic keeps background admission across context construction, inference
and destruction. It shares the decoder through helpers that run under an already-held slot,
without recursively acquiring an interactive slot. Its whole-call cancellation/preemption
limitation remains unchanged; this does not enable the rejected speech masks in the app.

A new explicit native regression holds an idle scheduler slot and proves that detection and
destruction both queue before completing. It fails before the fix with both operations bypassing
the slot and passes afterward; it does not run two native engines concurrently to provoke a
crash. The existing native speech-diagnostic check also passes for both profiles without deadlock.
Checks pass 96 feature voice tests (6 ignored), 57 default voice tests (2 ignored), 23 spike tests,
the two explicit native tests and all-feature Clippy, formatting and repository guards. The
normal voice suites run with the pinned real Silero model, retaining their speech/pause/silence/
noise fixture checks. Only the existing model is downloaded into a disposable directory; no
personal audio/profile is used. The app stays closed and temporary models/logs are removed.
This establishes native ownership; human speech-quality and real-meeting acceptance remain open.

## Buffered WAV input

The shared WAV reader uses the standard library's bounded input buffer, so sample-by-sample
parsing does not require a backing file read for every sample. This covers transcription,
native speaker analysis and the offline speech diagnostics. It adds no dependency or setting;
PCM chunk limits, source clocks, validation and cancellation behavior stay the same.

A counting-reader regression records 96,017 backing reads before the fix for one second of
48 kHz stereo float32 input. The buffered path passes a limit of 407 reads while producing
exactly the reference mono/resampled PCM and the full 16,000-sample clock. This is an I/O-call
check, not an end-to-end latency or real-device performance qualification. Existing malformed,
truncated, cancellation and chunk-boundary checks also pass.

Validation passes 99 feature voice tests (6 ignored), 60 default voice tests (2 ignored),
23 spike tests, all-feature Clippy, formatting and repository guards. Synthetic inputs only;
no native inference, audio/model downloads or app startup is needed for this check. Temporary
logs are removed and personal/shared caches are preserved. Human speech-quality, physical
source-clock and long-session acceptance remain open.
