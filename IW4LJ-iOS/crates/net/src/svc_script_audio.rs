use crate::{ReliableEventHub, ReliableRow, WireError, WireReader, WireWriter};
use bevy::prelude::*;
use sim::{ClientId, ScriptAudioCommand};

#[derive(Message, Clone, Debug, PartialEq)]
pub struct SvcScriptAudio(pub ScriptAudioCommand);

pub(crate) fn fanout_script_audio(
    reliable: &mut ReliableEventHub,
    clients: impl IntoIterator<Item = ClientId>,
    commands: &[ScriptAudioCommand],
) {
    for client in clients {
        for command in commands {
            if command.target().is_some_and(|target| target != client) {
                continue;
            }
            reliable
                .queue_mut(client)
                .push(ReliableRow::ScriptAudio(command.clone()));
        }
    }
}

pub(crate) fn encode_script_audio(out: &mut WireWriter, command: &ScriptAudioCommand) {
    match command {
        ScriptAudioCommand::ChannelVolumes {
            client,
            priority,
            volumes,
            fade_ms,
        } => {
            out.put_u8(3);
            out.put_u32(client.0);
            out.put_u8(*priority);
            out.put_i32(*fade_ms);
            match volumes {
                None => out.put_u8(0),
                Some(volumes) => {
                    out.put_u8(volumes.len() as u8);
                    for (name, gain) in volumes {
                        out.put_u8(name.len() as u8);
                        out.put_bytes(name.as_bytes());
                        out.put_f32(*gain);
                    }
                }
            }
        }
        ScriptAudioCommand::DeactivateChannelVolumes {
            client,
            priority,
            fade_ms,
        } => {
            out.put_u8(4);
            out.put_u32(client.0);
            out.put_u8(*priority);
            out.put_i32(*fade_ms);
        }
        ScriptAudioCommand::MusicPlay(alias) => {
            out.put_u8(0);
            out.put_u16(alias.len() as u16);
            out.put_bytes(alias.as_bytes());
        }
        ScriptAudioCommand::SoundFade { volume, fade_ms } => {
            out.put_u8(2);
            out.put_f32(*volume);
            out.put_i32(*fade_ms);
        }
        ScriptAudioCommand::MusicStop { fade_ms } => {
            out.put_u8(1);
            out.put_i32(*fade_ms);
        }
    }
}

pub(crate) fn decode_script_audio(
    input: &mut WireReader<'_>,
) -> Result<ScriptAudioCommand, WireError> {
    let command = match input.get_u8()? {
        3 => {
            let client = ClientId(input.get_u32()?);
            let priority = input.get_u8()?;
            let fade_ms = input.get_i32()?;
            let count = input.get_u8()?;
            if count > 64 {
                return Err(WireError::Malformed("too many channel gains"));
            }
            let mut volumes = std::collections::BTreeMap::new();
            for _ in 0..count {
                let len = input.get_u8()? as usize;
                if len > 64 {
                    return Err(WireError::Malformed("channel name exceeds limit"));
                }
                let mut bytes = vec![0; len];
                input.get_bytes(&mut bytes)?;
                let name = String::from_utf8(bytes)
                    .map_err(|_| WireError::Malformed("channel name is not UTF-8"))?;
                let gain = input.get_f32()?;
                if volumes.insert(name, gain).is_some() {
                    return Err(WireError::Malformed("duplicate channel gain"));
                }
            }
            ScriptAudioCommand::ChannelVolumes {
                client,
                priority,
                volumes: (count > 0).then_some(volumes),
                fade_ms,
            }
        }
        4 => ScriptAudioCommand::DeactivateChannelVolumes {
            client: ClientId(input.get_u32()?),
            priority: input.get_u8()?,
            fade_ms: input.get_i32()?,
        },
        0 => {
            let len = input.get_u16()? as usize;
            if len > 1023 {
                return Err(WireError::Malformed("music alias exceeds limit"));
            }
            let mut bytes = vec![0; len];
            input.get_bytes(&mut bytes)?;
            ScriptAudioCommand::MusicPlay(
                String::from_utf8(bytes)
                    .map_err(|_| WireError::Malformed("music alias is not UTF-8"))?,
            )
        }
        2 => ScriptAudioCommand::SoundFade {
            volume: input.get_f32()?,
            fade_ms: input.get_i32()?,
        },
        1 => ScriptAudioCommand::MusicStop {
            fade_ms: input.get_i32()?,
        },
        _ => return Err(WireError::Malformed("unknown script audio command")),
    };
    if !command.valid() {
        return Err(WireError::Malformed("invalid script audio command"));
    }
    Ok(command)
}
