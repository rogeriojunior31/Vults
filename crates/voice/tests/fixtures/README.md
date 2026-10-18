# Voice fixtures

All WAV files are 16 kHz mono PCM16 with a canonical 44-byte header. No user recordings or
models are included. Tests do not download anything.

The speech excerpt is samples 6400..36800 (0.4..2.3 seconds) of
[`samples/jfk.wav` from whisper.cpp v1.8.5](https://github.com/ggml-org/whisper.cpp/blob/v1.8.5/samples/jfk.wav),
source SHA-256 `59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e`.
It is from President Kennedy's 1961 inaugural address, a US federal government speech in the
public domain; the [National Archives](https://www.archives.gov/milestone-documents/president-john-f-kennedys-inaugural-address)
provides the address's historical context.

- `speech.wav`: 0.5 s silence, excerpt, 1 s silence.
- `pauses.wav`: 0.5 s silence, excerpt, 0.15 s silence, excerpt, 0.75 s silence, excerpt,
  1 s silence. The short pause stays inside the first segment; the long pause separates it.
- `silence.wav`: 1 s digital silence.
- `breath-noise.wav`: 2 s synthetic noise, not a human breath recording. Python's
  `random.Random(17)` supplies uniform samples in [-1, 1], multiplied by 3276 and by the
  triangular envelope `max(0, 1 - abs(i / 16000 - 0.7) / 0.16)`, then rounded to PCM16.
  This covers a brief breath-like burst, not every kind of breathing or microphone noise.

Reference edges are detector regression baselines, not hand-labelled linguistic boundaries.
They use the installed catalog's `ggml-silero-v6.2.0.bin`, SHA-256
`2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987`, with a 40 ms
tolerance (one 32 ms model frame plus rounding). Set `VOICE_TEST_VAD` to that model's path
and run `cargo test -p vults-voice wav_fixtures`. Without it the model tests skip; the pure
hysteresis, transition/reset and synthetic Chat pause tests still run.

| Fixture | SHA-256 |
| --- | --- |
| speech.wav | b2f803cd887a4d3c41542dcf3540772a526e64ef82138b17663259c367579dad |
| pauses.wav | d7ed46b18796c34b2ad073583fd0a338f7c20c4f4b5b9b06bc83c643fb4243be |
| silence.wav | 643f8a8dc8bd9c19225afffad2becfec5426180b3749cb208abdf1a6c8354efc |
| breath-noise.wav | 36c908fd93f322e351ea80eda3928600352e87b158c46d55021cd953ed390d2b |


## Transcript fixtures

`transcript-loops.tsv` contains synthetic regression inputs and expected outputs, separated by
one tab. It covers case/punctuation variants, Portuguese, Spanish, English and unspaced Chinese.
These are deterministic text examples, not claims about model accuracy.

`reported-music-loop.txt` is a short excerpt of glangford's reported `large-v2` Run 2 output
in [Whisper discussion 1873](https://github.com/openai/whisper/discussions/1873), posted on
2023-12-08. Segment timestamps were removed and the text was joined with spaces. It retains
five occurrences of the first phrase and three of the second. Both are below our six-repeat
threshold and deliberately survive: the filter cannot remove every hallucination without
also removing ordinary speech. This is a public reported output, not a locally reproduced
inference or a private recording.

## Dictionary vocabulary WER

`vocabulary.jsonl` contains 20 original sentences per language (English and Brazilian Portuguese),
including ordinary sentences without a dictionary name. The same 16-name dictionary is supplied
to every clip, not a clue selected from the sentence being decoded. JSON uses Unicode escapes
for the Portuguese text. No human recordings or personal data are included.

The ignored test reads canonical 16 kHz mono PCM16 WAV files named after each `id` from
`VOICE_WER_CORPUS`. Supply real, consented recordings for human-speech evidence. Alternatively,
`VOICE_WER_SYNTHESIZE=1` explicitly generates missing clips offline with installed `espeak-ng`
(`en-us`/`pt-br`, 155 words/minute) and `ffmpeg`. These are synthetic fixtures and do not measure
human accents or microphone conditions. Use a fresh corpus directory after changing sentences.
No test downloads a model, starts Vults or uses the microphone.

```sh
VOICE_WER_MODELS=/path/to/installed/models \
VOICE_WER_CORPUS=/path/to/local/corpus \
VOICE_WER_SYNTHESIZE=1 \
cargo test -p vults-voice --features vulkan vocabulary_wer_on_twenty_sentences_per_language -- --ignored --nocapture
```

Omit the synthesis flag for supplied recordings. `VOICE_WER_MODEL` optionally selects exactly
`base`, `small` or `turbo`; an invalid id fails. `VOICE_WER_MANIFEST` may select a separate reviewed
40-line JSONL manifest with the same schema. Every selected model must exist locally. The runner
compares the production baseline against the combined coding seed plus dictionary context,
counts case/punctuation-normalized word substitutions, insertions and deletions, and reports
micro-averaged errors/word totals per model/language. No post-transcription Your words rules run.
The real model tokenizer also checks a long list against the shared 224-token budget.

Silence and synthetic breath/noise must stay empty through both the public transcription path
and the decoder without amplitude trimming. An enabled production pair fails the ignored check
if WER does not improve strictly or a silence check produces text. Per-call context never changes
the cached model or Chat. The default preference remains off; a synthetic comparison is not a
claim of improved human-speech WER. Human desktop acceptance remains separate.

### Measured on 2026-10-18

Local eSpeak NG 1.52.0 speech, 155 words/minute, converted with ffmpeg, decoded through Vulkan.
No Silero model was passed, so the same production amplitude trim served both sentence paths.
The silence column requires empty output for both fixtures, including direct decoding without
that trim. All dictionaries used the manifest's same 16 names. Counts are word edit errors
over total reference words; insertions can make WER exceed 100%.

| Model | Language | Baseline errors/words (WER) | Dictionary errors/words (WER) | Silence/noise empty | Dictionary gate |
| --- | --- | --- | --- | --- | --- |
| Base q5_1 | pt | 161/132 (121.97%) | 121/132 (91.67%) | No | Blocked |
| Base q5_1 | en | 52/136 (38.24%) | 39/136 (28.68%) | No | Blocked |
| Small q5_1 | pt | 113/132 (85.61%) | 102/132 (77.27%) | No | Blocked |
| Small q5_1 | en | 33/136 (24.26%) | 17/136 (12.50%) | No | Blocked |
| Turbo q5_0 | pt | 63/132 (47.73%) | 55/132 (41.67%) | No | Blocked |
| Turbo q5_0 | en | 19/136 (13.97%) | 8/136 (5.88%) | Yes | Qualified, preference off by default |

Raw digital silence generated text with Base/pt, Base/en, Small/en and Turbo/pt. Breath-like
noise generated text with Base and Small in both languages. No failures were filtered out or
thresholds relaxed to qualify a pair. The Turbo rerun reproduced both language totals with the
enabled-pair assertion active; it also checked preview/final equality, cancellation and baseline
restoration for both an empty dictionary and the Chat path on the same cached model.

| Local input | SHA-256 |
| --- | --- |
| vocabulary.jsonl | fc1fdf42243b1d74e5ed09599196a37cbdaa9579f95a0839c9c3ebf192555ec3 |
| ggml-base-q5_1.bin | 422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898 |
| ggml-small-q5_1.bin | ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb |
| ggml-large-v3-turbo-q5_0.bin | 394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2 |

These invented-name fixtures are a narrow local regression corpus, not a representative
benchmark. The poor synthetic Portuguese results do not establish real Portuguese accuracy.
Human speech, accents, microphones and desktop delivery still need their separate acceptance.
