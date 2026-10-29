//! Explicit offline smart-turn v3.0 inference. No capture, downloads or app activation.
mod control;
mod features;

use crate::onnx::{NativeSession, Runtime};
use ort::{
    session::{RunOptions, Session, builder::GraphOptimizationLevel},
    value::Tensor,
};
use std::{
    mem::ManuallyDrop,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

// Quantized scores vary across runtime versions; qualify this exact CPU library independently.
pub const RUNTIME_SHA256: &str = "469849977cb60744ea160927e1a3a0643fd4c55c0aff743251c6d293402e334e";
pub const MODEL_SIZE: usize = 8_757_193;
pub const MODEL_SHA256: &str = "07a133aba31e2d0b523f17f8c2e4e65efe6d8f685efd12ca4fe21ebf4e798991";
pub const MODEL_REVISION: &str = "f766f81d3cfdf7737ac64aad813d91bbfd56bf93";
pub const MODEL_URL: &str = "https://huggingface.co/pipecat-ai/smart-turn-v3/resolve/f766f81d3cfdf7737ac64aad813d91bbfd56bf93/smart-turn-v3.0.onnx";

pub struct Detector {
    session: NativeSession,
    features: features::Features,
}
impl std::fmt::Debug for Detector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Detector")
    }
}
impl Detector {
    /// Supply the admitted shared runtime and an exact pinned model. Model construction cannot abort;
    /// cancellation discards its session before publication. Run on the owner's bounded worker.
    pub fn load(runtime: &Runtime, path: &Path, stop: &AtomicBool) -> Result<Option<Self>, String> {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        if runtime.fingerprint() != RUNTIME_SHA256 {
            return Err("semantic turn requires the qualified ONNX runtime".into());
        }
        let Some(bytes) = crate::model_file::read_file(path, MODEL_SIZE, MODEL_SHA256, stop)? else {
            return Ok(None);
        };
        let session = {
            let Some(_busy) = crate::WHISPER.interactive(stop, None) else {
                return Ok(None);
            };
            if stop.load(Ordering::SeqCst) {
                return Ok(None);
            }
            let session = Session::builder(&runtime.environment()?)
                .map_err(|e| e.to_string())?
                .with_optimization_level(GraphOptimizationLevel::All)
                .map_err(|e| e.to_string())?
                .with_intra_threads(1)
                .map_err(|e| e.to_string())?
                .with_inter_threads(1)
                .map_err(|e| e.to_string())?
                .with_memory_pattern(false)
                .map_err(|e| e.to_string())?
                .commit_from_memory(&bytes)
                .map_err(|e| e.to_string())?;
            NativeSession(ManuallyDrop::new(session))
        };
        if session.0.inputs().len() != 1
            || session.0.inputs()[0].name() != "input_features"
            || session.0.outputs().len() != 1
            || session.0.outputs()[0].name() != "logits"
        {
            return Err("unexpected semantic turn model contract".into());
        }
        let features = features::Features::new();
        Ok((!stop.load(Ordering::SeqCst)).then_some(Self { session, features }))
    }
    /// Mono 16 kHz audio; only its last eight seconds are evaluated. Pass the policy ticket's
    /// remaining budget and cancellation flag; the result must retain that ticket/generation.
    /// Timeouts/errors disable semantics through EndOfTurn; cancellation is not an inference failure.
    pub fn score(
        &mut self,
        audio: &[f32],
        stop: &AtomicBool,
        budget: Duration,
    ) -> Result<Option<f32>, String> {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        if budget.is_zero() {
            return Err("semantic turn inference budget expired".into());
        }
        let deadline = Instant::now() + budget.min(super::end::SEMANTIC_HOLD);
        let Some(features) = self.features.extract(audio, stop, Some(deadline))? else {
            return Ok(None);
        };
        let Some(_busy) = crate::WHISPER.interactive(stop, Some(deadline)) else {
            return if stop.load(Ordering::SeqCst) {
                Ok(None)
            } else {
                Err("semantic turn inference budget expired".into())
            };
        };
        control::run(
            stop,
            deadline,
            || RunOptions::new().map_err(|e| e.to_string()),
            |options| options.terminate().map_err(|e| e.to_string()),
            |options| {
                let input = Tensor::from_array(([1, features::MELS, features::FRAMES], features))
                    .map_err(|e| e.to_string())?;
                let outputs = self
                    .session
                    .0
                    .run_with_options(ort::inputs!["input_features" => input], options)
                    .map_err(|e| e.to_string())?;
                let (shape, data) = outputs["logits"]
                    .try_extract_tensor::<f32>()
                    .map_err(|e| e.to_string())?;
                if shape.as_ref() != [1, 1]
                    || data.len() != 1
                    || !data[0].is_finite()
                    || !(0.0..=1.0).contains(&data[0])
                {
                    return Err("invalid semantic turn probability".into());
                }
                Ok(data[0])
            },
        )
    }
}
#[cfg(test)]
mod tests;
