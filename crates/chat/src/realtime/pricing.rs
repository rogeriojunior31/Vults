use serde::Deserialize;

// Qualified against the linked model pages on 2026-10-19. Recheck before runtime activation.
pub(super) const REVIEWED_MS: i64 = 1_792_368_000_000;
const VALID_MS: i64 = 30 * 86_400_000;
pub(super) const CONTEXT: u64 = 128_000;

#[derive(Clone, Copy, Debug)]
pub(super) struct Rates {
    // Nano-USD per token: preserve fractional micro-USD until the final upward rounding.
    text: u64,
    cached_text: u64,
    audio: u64,
    cached_audio: u64,
    output_text: u64,
    output_audio: u64,
}

impl Rates {
    pub(super) fn for_model(model: &str) -> Option<(&'static str, Self)> {
        let (model, values) = match model {
            "gpt-realtime-2.1" => ("gpt-realtime-2.1", [4000, 400, 32000, 400, 24000, 64000]),
            "gpt-realtime-2.1-mini" => ("gpt-realtime-2.1-mini", [600, 60, 10000, 300, 2400, 20000]),
            _ => return None,
        };
        let [text, cached_text, audio, cached_audio, output_text, output_audio] = values;
        Some((
            model,
            Self {
                text,
                cached_text,
                audio,
                cached_audio,
                output_text,
                output_audio,
            },
        ))
    }

    pub(super) fn ceiling(self, post_instructions: u64, output: u64, now_ms: i64) -> Option<u64> {
        if !(REVIEWED_MS..REVIEWED_MS + VALID_MS).contains(&now_ms) {
            return None;
        }
        // Instructions are outside post_instructions. Cover the whole context as text PLUS
        // the entire bounded conversation as audio: deliberate double counting, no tokenizer guess.
        micros([
            (CONTEXT, self.text),
            (post_instructions, self.audio),
            (output, self.output_text.max(self.output_audio)),
        ])
    }

    pub(super) fn cost(self, usage: &Usage) -> Option<u64> {
        let input = &usage.input_token_details;
        let cached = input.cached_tokens_details.as_ref();
        let (text, audio) = cached.map_or((0, 0), |c| (c.text_tokens, c.audio_tokens));
        if usage.input_tokens.checked_add(usage.output_tokens)? != usage.total_tokens
            || input.text_tokens.checked_add(input.audio_tokens)? != usage.input_tokens
            || usage
                .output_token_details
                .text_tokens
                .checked_add(usage.output_token_details.audio_tokens)?
                != usage.output_tokens
            || input.image_tokens != 0
            || (input.cached_tokens != 0 && cached.is_none())
            || cached.is_some_and(|c| c.image_tokens != 0)
            || text.checked_add(audio)? != input.cached_tokens
        {
            return None;
        }
        micros([
            (input.text_tokens.checked_sub(text)?, self.text),
            (text, self.cached_text),
            (input.audio_tokens.checked_sub(audio)?, self.audio),
            (audio, self.cached_audio),
            (usage.output_token_details.text_tokens, self.output_text),
            (usage.output_token_details.audio_tokens, self.output_audio),
        ])
    }
}

fn micros<const N: usize>(parts: [(u64, u64); N]) -> Option<u64> {
    let nanos = parts.into_iter().try_fold(0_u128, |sum, (tokens, rate)| {
        sum.checked_add(u128::from(tokens).checked_mul(u128::from(rate))?)
    })?;
    let amount = u64::try_from(nanos.div_ceil(1000)).ok()?;
    (amount <= i64::MAX as u64).then_some(amount)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Usage {
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub input_token_details: Input,
    pub output_token_details: Tokens,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Input {
    pub text_tokens: u64,
    pub audio_tokens: u64,
    #[serde(default)]
    pub image_tokens: u64,
    cached_tokens: u64,
    cached_tokens_details: Option<Cached>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cached {
    text_tokens: u64,
    audio_tokens: u64,
    #[serde(default)]
    image_tokens: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Tokens {
    pub text_tokens: u64,
    pub audio_tokens: u64,
}
