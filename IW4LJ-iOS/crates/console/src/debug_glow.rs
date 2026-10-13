use crate::debug_scalar::debug_scalars;
use crate::{ConsoleCommand, ConsoleLine, ConsoleRegistry, ConsoleSettings, ConsoleState};
use bevy::prelude::*;
use render_frontend::assemble::drawsurf::dof::GlowDvars;

debug_scalars! {
    GlowDvars;
    "r_glow" { path: [enable], min: 0.0, max: 1.0, kind: bool },
    "r_glowUseTweaks" { path: [use_tweaks], min: 0.0, max: 1.0, kind: bool },
    "r_glowTweakEnable" { path: [tweak_enable], min: 0.0, max: 1.0, kind: bool },
    "r_glowTweakRadius0" { path: [tweak_radius], min: 0.0, max: 32.0, kind: float },
    "r_glowTweakBloomIntensity0" { path: [tweak_intensity], min: 0.0, max: 20.0, kind: float },
    "r_glowTweakBloomCutoff" { path: [tweak_cutoff], min: 0.0, max: 1.0, kind: float },
    "r_glowTweakBloomDesaturation" { path: [tweak_desaturation], min: 0.0, max: 1.0, kind: float },
    "r_glow_allowed" { path: [allowed], min: 0.0, max: 1.0, kind: bool },
    "r_glow_allowed_script_forced" { path: [allowed_script_forced], min: 0.0, max: 1.0, kind: bool },
}

pub(crate) fn register(registry: &mut ConsoleRegistry) {
    for &name in NAMES {
        if registry.resolve(name).is_none() {
            registry
                .register(crate::CommandSpec::new(name).usage(format!("{name} [value] — glow")));
        }
    }
}

pub(crate) fn route(
    mut commands: MessageReader<ConsoleCommand>,
    mut dvars: ResMut<GlowDvars>,
    mut console: ResMut<ConsoleState>,
    settings: Res<ConsoleSettings>,
    mut line: ResMut<ConsoleLine>,
) {
    for cmd in commands.read() {
        let name = cmd.name.as_str();
        if !NAMES.contains(&name) {
            continue;
        }
        let parsed = match cmd.args.as_slice() {
            [] => Ok(None),
            [value] => value
                .parse::<f32>()
                .ok()
                .filter(|v| v.is_finite())
                .map(Some)
                .ok_or(()),
            _ => Err(()),
        };
        let (current, min, max, is_bool) = scalar_current(&dvars, name);
        let msg = match parsed {
            Ok(None) => format!("{name} = {current} (domain {min}..{max})"),
            Ok(Some(value))
                if (min..=max).contains(&value) && (!is_bool || value == 0.0 || value == 1.0) =>
            {
                scalar_assign(&mut dvars, name, value);
                format!("{name} = {value}")
            }
            _ => format!("usage: {name} [finite value {min}..{max}]"),
        };
        diag::info!(Console, "{msg}");
        line.0 = msg.clone();
        console.echo(msg, settings.log_capacity);
    }
}
