use super::*;
use vults_audio::playback::{Cursor, Playback};
use vults_voice::turn::BargeIn;

pub(super) enum Choice {
    Default,
    #[cfg(target_os = "linux")]
    Routed(vults_audio::echo::duplex::Route),
}
impl Choice {
    #[cfg(target_os = "linux")]
    pub fn detects(&self) -> bool {
        matches!(self, Self::Routed(route) if route.chosen_policy() != vults_audio::echo::duplex::Interruption::Disabled)
    }
    pub fn captures(&self) -> bool {
        !matches!(self, Self::Default)
    }
}
pub(super) enum Devices {
    Default(Arc<Lease>),
    #[cfg(target_os = "linux")]
    Routed(vults_audio::echo::duplex::Session),
}
impl Devices {
    pub fn open(
        choice: Choice,
        lease: Arc<Lease>,
        active: Arc<dyn Fn() -> bool + Send + Sync>,
    ) -> Result<Self, Failure> {
        if !active() {
            return Err(Failure::Cancelled);
        }
        match choice {
            Choice::Default => Ok(Self::Default(lease)),
            #[cfg(target_os = "linux")]
            Choice::Routed(route) => {
                vults_audio::echo::duplex::Session::start_guarded(route, Arc::default(), lease, active)
                    .map(Self::Routed)
                    .map_err(|e| match e {
                        vults_audio::echo::duplex::Failure::Busy => Failure::Busy,
                        _ => Failure::Output,
                    })
            }
        }
    }
    pub fn ready(&self) -> Result<bool, Failure> {
        match self {
            Self::Default(_) => Ok(true),
            #[cfg(target_os = "linux")]
            Self::Routed(s) => s.ready().map_err(|_| Failure::Output),
        }
    }
    pub fn interruption(&self) -> BargeIn {
        match self {
            Self::Default(_) => BargeIn::Disabled,
            #[cfg(target_os = "linux")]
            Self::Routed(s) => match s.chosen_policy() {
                vults_audio::echo::duplex::Interruption::Disabled => BargeIn::Disabled,
                vults_audio::echo::duplex::Interruption::Headphones => BargeIn::Headphones,
                vults_audio::echo::duplex::Interruption::EchoCancelled => BargeIn::EchoCancelled,
            },
        }
    }
    pub fn input(&self) -> Result<Option<Vec<i16>>, Failure> {
        match self {
            Self::Default(_) => Ok(None),
            #[cfg(target_os = "linux")]
            Self::Routed(s) => s.take_input().map_err(|_| Failure::Output),
        }
    }
    pub fn output(&self, cancel: Arc<AtomicBool>) -> Result<Output, Failure> {
        match self {
            Self::Default(lease) => Playback::start_with_owner(cancel, lease.clone())
                .map(Output::Default)
                .map_err(|e| match e {
                    vults_audio::playback::Failure::Busy => Failure::Busy,
                    _ => Failure::Output,
                }),
            #[cfg(target_os = "linux")]
            Self::Routed(s) => s.output().map(Output::Routed).map_err(|e| match e {
                vults_audio::echo::duplex::Failure::Busy => Failure::Busy,
                _ => Failure::Output,
            }),
        }
    }
}
pub(super) enum Output {
    Default(Playback),
    #[cfg(target_os = "linux")]
    Routed(vults_audio::echo::duplex::Output),
}
impl playback::Output for Output {
    fn push(&self, pcm: &[i16]) -> Result<(), Failure> {
        match self {
            Self::Default(p) => p.push(pcm).map_err(|_| Failure::Output),
            #[cfg(target_os = "linux")]
            Self::Routed(p) => p.push(pcm).map_err(|_| Failure::Output),
        }
    }
    fn finish(&self) -> Result<(), Failure> {
        match self {
            Self::Default(p) => p.finish().map_err(|_| Failure::Output),
            #[cfg(target_os = "linux")]
            Self::Routed(p) => p.finish().map_err(|_| Failure::Output),
        }
    }
    fn cursor(&self) -> Cursor {
        match self {
            Self::Default(p) => p.cursor(),
            #[cfg(target_os = "linux")]
            Self::Routed(p) => p.cursor(),
        }
    }
    fn stop(&self) -> Cursor {
        match self {
            Self::Default(p) => p.stop(),
            #[cfg(target_os = "linux")]
            Self::Routed(p) => p.stop(),
        }
    }
    fn route_verified(&self) -> bool {
        match self {
            Self::Default(_) => true,
            #[cfg(target_os = "linux")]
            Self::Routed(p) => p.route_verified(),
        }
    }
}
