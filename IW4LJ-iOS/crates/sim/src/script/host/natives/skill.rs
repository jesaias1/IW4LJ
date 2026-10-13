use super::super::args::{arg, float, int, string};
use crate::script::{Namespace, NativeRegistry, Value};

pub(crate) fn register(registry: &mut NativeRegistry) {
    registry.register(
        Namespace::Function,
        "setplayerteamrank",
        |world, _, args| {
            if args.len() != 3 {
                return Err("setplayerteamrank needs a player, ranking group and score".into());
            }
            super::player::check_data_write(world)?;
            let client = super::player::player(world, arg(args, 0)?)?;
            let group = int(args, 1)?;
            let score = int(args, 2)?;
            if group < 0 {
                return Err("ranking group must be nonnegative".into());
            }
            let mut runtime = world.resource_mut::<crate::script::RoundScript>();
            if !runtime.ranks_sent {
                runtime.team_ranks.insert(client, (group, score));
            }
            Ok(Value::Undefined)
        },
    );
    registry.register(Namespace::Function, "sendranks", |world, _, args| {
        if !args.is_empty() {
            return Err("sendranks takes no arguments".into());
        }
        super::player::check_data_write(world)?;
        let runtime = world.resource::<crate::script::RoundScript>();
        if runtime.ranks_sent {
            return Ok(Value::Undefined);
        }
        let ranks = runtime.team_ranks.clone();
        let mode = crate::frame::FrameWorld::from_world(world)
            .game_mode_kind()
            .token();
        let count = world
            .resource_mut::<crate::PersistentDataStore>()
            .submit_ranks(mode, &ranks)
            .map_err(|error| format!("skill rating: {error:?}"))?;
        let mut runtime = world.resource_mut::<crate::script::RoundScript>();
        runtime.team_ranks.clear();
        runtime.ranks_sent = true;
        diag::info!(
            Sim,
            "script rankings submitted: mode={} accounts={}",
            mode,
            count
        );
        Ok(Value::Undefined)
    });
    registry.register(Namespace::Function, "updateskill", |world, _, args| {
        if args.len() != 4 {
            return Err("updateskill needs two players, a mode and a score".into());
        }
        super::player::check_data_write(world)?;
        let first = crate::ClientId(super::player::player(world, arg(args, 0)?)?);
        let second = crate::ClientId(super::player::player(world, arg(args, 1)?)?);
        let mode = string(args, 2)?;
        let score = float(args, 3)?;
        world
            .resource_mut::<crate::PersistentDataStore>()
            .update_skill(first, second, &mode, score)
            .map_err(|error| format!("skill rating: {error:?}"))?;
        Ok(Value::Undefined)
    });
}
