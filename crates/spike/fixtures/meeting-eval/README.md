# Synthetic scorer fixtures

These JSON inputs contain invented anonymous timelines, not speech or human annotations. Their
all-zero WAV hash is a placeholder and must not be used with the native spike. The `en` and `pt`
tags exercise report metadata only; they do not measure either spoken language.

- `en-*`: a perfect one-to-one permutation of two labels; DER 0.
- `pt-*`: the same two reference speakers, but a fragmented/merged prediction; globally optimal
  matching keeps 3,000 of 5,000 speaker-ms correct, with 2,000 confused speaker-ms and DER 0.4.

Both are evaluation-tagged synthetic examples. They are not a calibration/evaluation corpus or
release approval. The [bilingual reference](../../../../docs/reference/meeting-evaluation.md)
describes the fixed scoring profile, abstention semantics and independent source clocks.
