# Semantic turn model

V6.2.2 provides an **offline developer backend**, behind the `vults-voice` feature
`semantic-turn`. Normal app builds do not enable it. There is no download, microphone control,
Settings switch or network request in this backend. V6.7 adds the explicitly chosen conversation
preview below; ordinary builds still expose no semantic activation.
This is numerical qualification on the checked Linux x64 CPU runtime, not endpoint-quality,
pt-BR speech, latency-target or platform acceptance.

## Sources and licenses

The [smart-turn code and model documentation](https://github.com/pipecat-ai/smart-turn/tree/4786657e242dfe77dd138699ac564ee074a2a543)
and the [pinned weights card](https://huggingface.co/pipecat-ai/smart-turn-v3/blob/f766f81d3cfdf7737ac64aad813d91bbfd56bf93/README.md)
declare **BSD-2-Clause**. The earlier plan's Apache-2.0 description of these weights was incorrect.
The graph is an unmodified int8 v3.0 export; later v3.1/v3.2 exports are not substituted automatically.
Its [upstream license](https://github.com/pipecat-ai/smart-turn/blob/4786657e242dfe77dd138699ac564ee074a2a543/LICENSE)
is retained in [SMART-TURN-LICENSE.txt](../../crates/voice/assets/semantic-turn/SMART-TURN-LICENSE.txt).
No model weights are bundled with Vults.

The Rust feature extractor is independently implemented and tested against
[Transformers 4.57.1's WhisperFeatureExtractor](https://github.com/huggingface/transformers/blob/v4.57.1/src/transformers/models/whisper/feature_extraction_whisper.py)
and its [audio utilities](https://github.com/huggingface/transformers/blob/v4.57.1/src/transformers/audio_utils.py).
The reference code is Apache-2.0; its full license is retained in
[TRANSFORMERS-LICENSE.txt](../../crates/voice/assets/semantic-turn/TRANSFORMERS-LICENSE.txt).
The generated fixture arrays contain no model weights or newly recorded user audio.

## Pinned artifacts

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| [smart-turn-v3.0.onnx](https://huggingface.co/pipecat-ai/smart-turn-v3/resolve/f766f81d3cfdf7737ac64aad813d91bbfd56bf93/smart-turn-v3.0.onnx) | 8,757,193 | `07a133aba31e2d0b523f17f8c2e4e65efe6d8f685efd12ca4fe21ebf4e798991` |
| ONNX Runtime 1.31.0 Linux x64 CPU library | 29,345,384 | `469849977cb60744ea160927e1a3a0643fd4c55c0aff743251c6d293402e334e` |
| Retained smart-turn license | 1,301 | `0d66364067f678c08586ebb60a16a2aed4fa081ec11057df35585759ce0e774f` |
| Retained Transformers license | 11,418 | `77fd4710def9ec3c0f6225800e0235f15a425abd4a8b03559127fcd782612049` |

The runtime's exact archive, license, third-party notices and privacy document are recorded in
[Meeting model artifacts](meeting-models.md#download-pins). The shared `voice::onnx::Runtime`
owner requires an absolute trusted immutable library, an independent fingerprint and
`ORT_DISABLE_TELEMETRY=1` at process startup; it also disables the native telemetry API. One process
cannot switch its admitted runtime. The existing diarization Runtime path remains a re-export.

`Detector::load` additionally requires this qualified runtime fingerprint and exact model bytes.
The shared model reader refuses symlinks, nonregular files, size/hash mismatches and cancelled reads;
loading uses the verified in-memory bytes rather than reopening the path. Sessions use CPU execution,
one intra/inter-op thread and graph optimization `All`. An earlier Python 1.24.4 runtime produced a
different probability for the same short-input tensor; that run is not the reference. Changing the
runtime, graph settings, model or CPU target requires new qualification rather than relaxing tests.

## Feature contract

Input is mono f32 PCM at 16 kHz. Retain the last 128,000 samples; left-pad shorter audio with zeros
before zero-mean/unit-variance normalization (variance epsilon `1e-7`). Reject nonfinite samples in
the retained window and normalization overflow. No audio is written to disk by the backend.

Features use a periodic 400-sample Hann window, centered reflect padding, a 400-point FFT and
160-sample hop. The reference rounds complex FFT values to complex64 before float64 power and
Slaney-normalized 80-mel reduction. Apply a `1e-10` floor, log10, discard the final STFT frame, clamp
to the retained global maximum minus eight, then `(x + 4) / 4`. The model receives f32
`input_features` of shape `[1, 80, 800]`; `logits` must be a finite probability in `[0, 1]` with
shape `[1, 1]`. Despite its output name, no additional sigmoid is applied.

`voice::turn::end` owns the VAD/text readiness gate and threshold 0.5. A semantic score alone is not
speech detection or permission authority: even silence can receive a high completion probability.
The scheduler binds each request to its turn and ticket. Use `semantic_deadline(ticket)` to compute
the worker's remaining budget; do not restart its 1.2-second hold when dispatching late. The worker
checks cancellation/deadline during preprocessing, foreground queue admission and native inference.
RunOptions requests cooperative termination; already running native initialization/kernels cannot
promise instantaneous abort. Cancelled results are discarded; an error/timeout goes through the
policy's once-per-conversation failure fallback. The adapter retains ownership and rejects stale
results before publication. Session destruction shares the speech-work lock with other native work.

## Conversation preview integration

V6.7 connects this backend to the existing automatic-finish recording owner through the app's
optional `voice-semantic` Cargo feature, qualified only for Linux x64 GNU. The Nest reads a build
capability without opening files, devices, a keyring or a connection. Ordinary builds do not offer
this choice and native Start rejects it. In a preview build, first enable **Finish automatically
after a pause**, then explicitly select **Use local semantic turn detection (preview)** before
Start. It is off by default, frozen for the conversation and cleared on departure/end. Continuous
headphone/AEC routes retain explicit Finish and cannot enable this option.

There is no installer or download offer. An explicit developer must first provision the exact
checked artifacts from the table above under the app's data directory:
`voice/semantic-turn/libonnxruntime.so.1.31.0` and `voice/semantic-turn/smart-turn-v3.0.onnx`.
The native library must be trusted and immutable as required by the shared runtime owner. No path
or URL comes from the renderer. Explicit Start prepares files/model on a bounded worker before
opening the microphone. The page says the microphone is off during preparation. Preparation has
a 60-second limit; cancellation or expiry returns without opening capture, while the actual native
worker retains its lease until teardown. Missing/invalid files select visible silence fallback.
Only later real VAD speech, local words and the silence gate authorize audio scoring. A different runtime already
pinned by another preview backend is refused; the process never silently switches libraries.
Artifact distribution and endpoint/pt-BR/device qualification remain separate work.

One worker/model belongs to the explicitly started conversation. It accepts only one score at a
time, snapshots at most the last eight seconds of owned 16 kHz audio and retains the desktop lease
through inference, model destruction and panic cleanup. Neither score requests nor partial words
become commands, provider input or microphone authority. Existing capture closure and final local
transcription still precede submission of a complete turn.

The recording watcher continues observing fresh VAD/text while scoring runs separately. New speech
cancels the old ticket before a returned score is considered; newer words recheck the silence gate.
After preparation, the original policy deadline bounds score queueing and inference together: no
late submission restarts the 1.2-second hold. Detection/capture latency remains separate. A valid low score may rescore at the existing 200 ms pace.
A failed/missing/expired score or unavailable checked files disable semantics for the remainder of
that conversation; the page shows that local silence detection is being used. Missing VAD/GPU
preview still falls back to explicit Finish. Cap, Finish, Cancel and page/epoch revocation keep
their existing authority and never wait for model teardown to return to the user.

```sh
cargo build -p vults-app --features voice-semantic --bin vults
cargo test -p vults-app --features meeting-speakers,voice-semantic
```

Controlled app-owner tests exercise bounds, one-job admission, cancellation, renewed speech,
word growth, deadline expiry, persistent fallback and teardown without a model, microphone or
provider. Native GUI checks use controlled capabilities/owners, not physical speech acceptance.

## Validation and remaining gates

Stored full-array oracles were generated with NumPy 2.2.6 and Transformers 4.57.1's NumPy CPU path,
following the pinned upstream crop/pad and `do_normalize=True` inference recipe. They cover a short
periodic waveform, a longer-than-eight-second deterministic noise stream and the existing speech WAV.
Silence has a separate exact `-1.5` oracle. Fixture metadata includes every feature SHA-256 and
probabilities from Python ONNX Runtime 1.31.0, CPU, one thread and graph optimization `All`.

Rust tests compare all 64,000 features per oracle with maximum absolute error below `1e-4`, then
an explicit native test compares all four probabilities within `2e-4`. That native test is ignored
in ordinary CI: it requires the explicitly supplied checked graph/runtime and never downloads them.
Pure tests cover cancellation, expired preprocessing, queue withdrawal, hung native-work termination,
panic cleanup, stale deadlines and model corruption without loading a native library.

```sh
cargo test -p vults-voice --features semantic-turn
cargo clippy -p vults-voice --all-targets --features semantic-turn,diarization -- -D warnings
# After explicitly supplying the checked graph and trusted runtime:
ORT_DISABLE_TELEMETRY=1 \
VULTS_SEMANTIC_TEST_RUNTIME=/absolute/path/libonnxruntime.so.1.31.0 \
VULTS_SEMANTIC_TEST_RUNTIME_SHA256=469849977cb60744ea160927e1a3a0643fd4c55c0aff743251c6d293402e334e \
VULTS_SEMANTIC_TEST_MODEL=/absolute/path/smart-turn-v3.0.onnx \
cargo test -p vults-voice --features semantic-turn \
  native_probabilities_match_the_reference_on_checked_artifacts -- --ignored --nocapture
```

Real conversational endpoint labels, pt-BR quality, device/performance acceptance and artifact
distribution remain open. This preview approves no app download offer or automatic listening.
