use super::args::{arg, float, int, optional, vector};
use super::entities::EntityKind;
use crate::frame::FrameWorld;
use crate::script::Namespace::{Function, Method};
use crate::script::{NativeRegistry, RoundScript, Value};
use bevy_ecs::prelude::World;
use glam::Vec3;

pub(crate) const ATTRACTOR_SLOTS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Anchor {
    Entity(u64),
    Point([f32; 3]),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Attractor {
    pub anchor: Anchor,
    attracts: bool,
    strength: f32,
    max_dist: f32,
}

pub(crate) fn target(
    world: &mut World,
    value: &Value,
    offset: [f32; 3],
) -> Result<crate::MissileTarget, String> {
    if let Value::Vector(point) = value {
        return Ok(crate::MissileTarget::Point(*point));
    }
    let runtime = world.resource::<RoundScript>();
    let (_, entity) = runtime
        .entity(value)
        .filter(|(id, _)| runtime.live(id))
        .ok_or("target is not an entity or vector")?;
    let number = entity.number;
    let client = runtime.player_client_of(value);
    let frame = FrameWorld::from_world(world);
    if let Some(client) = client {
        let client = crate::ClientId(client);
        let life = frame
            .client_meta(client)
            .ok_or("target player disconnected")?
            .life_sequence;
        return Ok(crate::MissileTarget::Player {
            client,
            life,
            offset,
        });
    }
    let entity = frame
        .entity_kernel()
        .current_ref(number)
        .map_err(|_| "target entity is not live")?;
    Ok(crate::MissileTarget::Entity { entity, offset })
}

fn guide(
    world: &mut World,
    receiver: &Value,
    edit: impl FnOnce(&mut crate::MissileGuide),
) -> Result<Value, String> {
    let runtime = world.resource::<RoundScript>();
    let (_, entity) = runtime
        .entity(receiver)
        .filter(|(id, _)| runtime.live(id))
        .ok_or("receiver is not a missile")?;
    let EntityKind::Missile(id) = entity.kind else {
        return Err("receiver is not a missile".into());
    };
    let number = entity.number;
    let mut frame = FrameWorld::from_world(world);
    let missile = frame
        .projectile_mut_by_number(number)
        .filter(|p| p.id == id && p.live)
        .ok_or("missile is not live")?;
    edit(&mut missile.guide);
    Ok(Value::Undefined)
}

fn create_attractor(
    world: &mut World,
    attracts: bool,
    anchor: Value,
    args: &[Value],
) -> Result<Value, String> {
    let anchor = match anchor {
        Value::Vector(point) => Anchor::Point(point),
        other => Anchor::Entity(super::natives::engine::entity_id(world, &other)?),
    };
    let strength = float(args, 1)?;
    let max_dist = float(args, 2)?;
    if max_dist <= 0.0 {
        return Err("parameter 3: maxDist must be greater than zero".into());
    }
    let mut runtime = world.resource_mut::<RoundScript>();
    let slots = &mut runtime.engine.attractors;
    let index = slots.iter().position(Option::is_none).ok_or_else(|| {
        format!("Ran out of attractor/repulsors.  Max allowed: {ATTRACTOR_SLOTS}")
    })?;
    slots[index] = Some(Attractor {
        anchor,
        attracts,
        strength,
        max_dist,
    });
    Ok(Value::Int(index as i32))
}

pub(crate) fn register(registry: &mut NativeRegistry) {
    registry.register(Function, "missile_createattractorent", |world, _, args| {
        create_attractor(world, true, arg(args, 0)?.clone(), args)
    });
    registry.register(
        Function,
        "missile_createattractororigin",
        |world, _, args| create_attractor(world, true, Value::Vector(vector(args, 0)?), args),
    );
    registry.register(Function, "missile_createrepulsorent", |world, _, args| {
        create_attractor(world, false, arg(args, 0)?.clone(), args)
    });
    registry.register(
        Function,
        "missile_createrepulsororigin",
        |world, _, args| create_attractor(world, false, Value::Vector(vector(args, 0)?), args),
    );
    registry.register(Function, "missile_deleteattractor", |world, _, args| {
        let index = usize::try_from(int(args, 0)?)
            .ok()
            .filter(|&index| index < ATTRACTOR_SLOTS)
            .ok_or("parameter 1: Invalid attractor or repulsor")?;
        world.resource_mut::<RoundScript>().engine.attractors[index] = None;
        Ok(Value::Undefined)
    });
    registry.register(Method, "missile_settargetent", |world, receiver, args| {
        super::natives::engine::entity_id(world, arg(args, 0)?)?;
        let offset = optional(args, 1, vector)?.unwrap_or([0.0; 3]);
        let aim = target(world, arg(args, 0)?, offset)?;
        guide(world, receiver, |g| {
            g.target = Some(aim);
            g.passed = false;
        })
    });
    registry.register(Method, "missile_settargetpos", |world, receiver, args| {
        let point = vector(args, 0)?;
        guide(world, receiver, |g| {
            g.target = Some(crate::MissileTarget::Point(point));
            g.passed = false;
        })
    });
    registry.register(Method, "missile_cleartarget", |world, receiver, _| {
        guide(world, receiver, |g| g.target = None)
    });
    registry.register(
        Method,
        "missile_setflightmodedirect",
        |world, receiver, _| {
            guide(world, receiver, |g| {
                g.top = false;
                g.stage = 0;
                g.passed = false;
            })
        },
    );
    registry.register(Method, "missile_setflightmodetop", |world, receiver, _| {
        guide(world, receiver, |g| {
            g.top = true;
            g.stage = 0;
            g.passed = false;
        })
    });
}

pub(crate) use crate::missile_guidance::turn_toward;

fn attract(world: &mut World, now: i32) {
    let slots: Vec<Attractor> = world
        .resource::<RoundScript>()
        .engine
        .attractors
        .iter()
        .flatten()
        .copied()
        .collect();
    if slots.is_empty() {
        return;
    }
    let mut anchors = Vec::with_capacity(slots.len());
    for slot in slots {
        let origin = match slot.anchor {
            Anchor::Point(point) => Some(Vec3::from_array(point)),
            Anchor::Entity(object) => match super::players::entity_field(world, object, "origin") {
                Value::Vector(o) => Some(Vec3::from_array(o)),
                _ => None,
            },
        };
        anchors.extend(origin.map(|origin| (slot, origin)));
    }
    let seconds = crate::MATCH_TICK_MS as f32 * 0.001;
    for projectile in crate::frame::collect_projectiles(world) {
        if !projectile.live {
            continue;
        }
        let steered = projectile.guide.target.is_some();
        let mut frame = FrameWorld::from_world(world);
        let rocket = frame
            .combat_facts_for(projectile.weapon)
            .is_some_and(|facts| facts.weap_type == super::weapons::WEAPTYPE_PROJECTILE);
        if steered || !rocket {
            continue;
        }
        let origin = Vec3::from_array(projectile.origin_at(now));
        let velocity = Vec3::from_array(projectile.velocity);
        let speed = velocity.length();
        let Some(forward) = velocity.try_normalize() else {
            continue;
        };
        let mut force = Vec3::ZERO;
        for (slot, anchor) in &anchors {
            let delta = *anchor - origin;
            let ahead = delta.dot(forward);
            if ahead <= 0.0 {
                continue;
            }
            let perp = delta - forward * ahead;
            let mut side = perp.length();
            let mut dir = if side < 1e-5 {
                if slot.attracts {
                    continue;
                }
                Vec3::NEG_Z
            } else {
                perp / side
            };
            if !slot.attracts && dir.z > 0.0 {
                dir = Vec3::NEG_Z;
                side = 0.0;
            }
            let distance = delta.length();
            if distance > slot.max_dist {
                continue;
            }
            let angle = (side.abs() / ahead).atan() * std::f32::consts::FRAC_2_PI;
            let mut push = (1.0 - distance / slot.max_dist)
                * slot.strength
                * if slot.attracts { angle } else { angle - 1.0 };
            if slot.attracts {
                push = push.min(speed * side / ahead / seconds);
            }
            force += dir * push;
        }
        if force == Vec3::ZERO {
            continue;
        }
        let Some(dir) = (velocity + force * seconds).try_normalize() else {
            continue;
        };
        let delta = entity_iw4::truncated_tr_delta((dir * speed).to_array());
        if let Some(projectile) = frame.projectile_mut_by_number(projectile.entnum) {
            projectile.velocity = delta;
            projectile.pos = entity_iw4::Trajectory {
                tr_time: now,
                tr_type: entity_iw4::TR_LINEAR,
                tr_duration: 0,
                tr_delta: delta,
                tr_base: origin.to_array(),
            };
            projectile.apos = entity_iw4::fire_missile_apos(dir.to_array());
        }
    }
}

pub(crate) fn advance(world: &mut World) {
    let now = crate::level_time_ms(world.resource::<crate::step::StepRequest>().tick);
    attract(world, now);
}
