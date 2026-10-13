#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptAmbient {
    pub alias: Option<String>,
    pub start_ms: i32,
    pub end_ms: i32,
}

impl ScriptAmbient {
    pub fn valid(&self) -> bool {
        self.start_ms >= 0
            && self.end_ms >= self.start_ms
            && self
                .alias
                .as_ref()
                .is_none_or(|name| !name.is_empty() && name.len() <= 1023)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ScriptAudioCommand {
    MusicPlay(String),
    MusicStop {
        fade_ms: i32,
    },
    SoundFade {
        volume: f32,
        fade_ms: i32,
    },
    ChannelVolumes {
        client: crate::ClientId,
        priority: u8,
        volumes: Option<std::collections::BTreeMap<String, f32>>,
        fade_ms: i32,
    },
    DeactivateChannelVolumes {
        client: crate::ClientId,
        priority: u8,
        fade_ms: i32,
    },
}

impl ScriptAudioCommand {
    pub fn target(&self) -> Option<crate::ClientId> {
        match self {
            Self::ChannelVolumes { client, .. } | Self::DeactivateChannelVolumes { client, .. } => {
                Some(*client)
            }
            _ => None,
        }
    }

    pub fn valid(&self) -> bool {
        match self {
            Self::MusicPlay(alias) => alias.len() <= 1023,
            Self::MusicStop { fade_ms } => *fade_ms >= 0,
            Self::ChannelVolumes {
                priority,
                volumes,
                fade_ms,
                ..
            } => {
                (1..=3).contains(priority)
                    && *fade_ms >= 0
                    && volumes.as_ref().is_none_or(|volumes| {
                        !volumes.is_empty()
                            && volumes.len() <= 64
                            && volumes.iter().all(|(name, gain)| {
                                !name.is_empty()
                                    && name.len() <= 64
                                    && *name == name.to_ascii_lowercase()
                                    && gain.is_finite()
                                    && (0.0..=1.0).contains(gain)
                            })
                    })
            }
            Self::DeactivateChannelVolumes {
                priority, fade_ms, ..
            } => (1..=3).contains(priority) && *fade_ms >= 0,
            Self::SoundFade { volume, fade_ms } => {
                volume.is_finite() && *volume >= 0.0 && *fade_ms >= 0
            }
        }
    }
}
