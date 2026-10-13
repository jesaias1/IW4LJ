use bevy::ecs::message::MessageCursor;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::state::{app::StatesPlugin, state::StateTransitionSystems};
use std::any::{TypeId, type_name};
use std::marker::PhantomData;

#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum MatchScope {
    #[default]
    Absent,
    Loading,
    Live,
}

#[derive(SubStates, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[source(MatchScope = MatchScope::Live)]
pub enum RoundPhase {
    #[default]
    Between,
    Playing,
}

#[derive(SubStates, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[source(MatchScope = MatchScope::Live)]
pub enum Timeline {
    #[default]
    Running,
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScopeCommit;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScopeRetired;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InMatch;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InRound;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScopeSet {
    Install,
    Derive,
    Announce,
    Release,
}

#[derive(Resource)]
pub struct ScopeEpoch<S: States>(pub u64, PhantomData<S>);

impl<S: States> Default for ScopeEpoch<S> {
    fn default() -> Self {
        Self(1, PhantomData)
    }
}

#[derive(Resource)]
pub struct Staging<R: Resource>(Option<R>);

impl<R: Resource> Default for Staging<R> {
    fn default() -> Self {
        Self(None)
    }
}

impl<R: Resource> Staging<R> {
    pub fn put(&mut self, value: R) {
        assert!(self.0.is_none(), "{} is already staged", type_name::<R>());
        self.0 = Some(value);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeKind {
    Resource,
    Staged,
    Message,
}

struct Row {
    id: TypeId,
    state: TypeId,
    label: String,
    name: &'static str,
    kind: ScopeKind,
    optional: bool,
    live: bool,
    install: fn(&mut World),
    retire: fn(&mut World),
    present: fn(&World) -> bool,
    staging_empty: fn(&World) -> bool,
    active: fn(&World, &str) -> bool,
    discard: fn(&mut World),
}

#[derive(Resource, Default)]
pub struct ScopeRegistry {
    rows: Vec<Row>,
    pub retirement_serial: u64,
    pub retired_resources: usize,
}

impl ScopeRegistry {
    pub fn lifetimes(&self) -> impl Iterator<Item = (&str, &str, ScopeKind, bool)> {
        self.rows
            .iter()
            .map(|row| (row.name, row.label.as_str(), row.kind, row.live))
    }

    pub fn rows(&self, world: &World) -> impl Iterator<Item = (&str, &str, ScopeKind, bool)> {
        self.rows
            .iter()
            .map(move |row| (row.name, row.label.as_str(), row.kind, (row.present)(world)))
    }
}

pub struct ScopePlugin;

impl Plugin for ScopePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<StatesPlugin>() {
            app.add_plugins(StatesPlugin);
        }
        app.init_state::<MatchScope>()
            .add_sub_state::<RoundPhase>()
            .add_sub_state::<Timeline>()
            .init_resource::<ScopeRegistry>()
            .init_resource::<crate::Retiring>()
            .init_resource::<ScopeEpoch<MatchScope>>()
            .init_resource::<ScopeEpoch<RoundPhase>>()
            .init_resource::<ScopeEpoch<Timeline>>()
            .add_systems(
                StateTransition,
                (
                    retire_state::<Timeline>,
                    retire_state::<RoundPhase>,
                    retire_state::<MatchScope>,
                )
                    .chain()
                    .in_set(ScopeRetired)
                    .in_set(StateTransitionSystems::TransitionSchedules),
            );
        for schedule in [
            First.intern(),
            PreUpdate.intern(),
            RunFixedMainLoop.intern(),
            FixedFirst.intern(),
            FixedPreUpdate.intern(),
            FixedUpdate.intern(),
            FixedPostUpdate.intern(),
            FixedLast.intern(),
            Update.intern(),
            PostUpdate.intern(),
            Last.intern(),
        ] {
            app.configure_sets(schedule, InMatch.run_if(in_state(MatchScope::Live)))
                .configure_sets(schedule, InRound.run_if(in_state(RoundPhase::Playing)));
        }
        app.configure_sets(
            PostUpdate,
            (
                crate::RenderSet::Anim,
                crate::RenderSet::FrontendAssemble,
                crate::RenderSet::FrontendPrepare,
                crate::RenderSet::Fx,
            )
                .run_if(in_state(MatchScope::Live)),
        );
        app.add_systems(
            Update,
            capture_census
                .after(crate::ClientSet::Effects)
                .before(crate::ClientEdge(9)),
        );
        configure_scope(app, MatchScope::Loading);
        configure_scope(app, MatchScope::Live);
        configure_scope(app, RoundPhase::Playing);
        configure_scope(app, Timeline::Running);
        if cfg!(debug_assertions) {
            app.add_systems(Last, verify_registry.after(ScopeCommit));
        }
    }
}

fn configure_scope<S: States>(app: &mut App, scope: S) {
    app.configure_sets(
        OnEnter(scope.clone()),
        (ScopeSet::Install, ScopeSet::Derive, ScopeSet::Announce).chain(),
    )
    .add_systems(
        OnEnter(scope.clone()),
        (move |world: &mut World| install_scope::<S>(world, &scope)).in_set(ScopeSet::Install),
    );
}

fn install_scope<S: States>(world: &mut World, scope: &S) {
    let label = format!("{scope:?}");
    let actions: Vec<_> = world
        .resource::<ScopeRegistry>()
        .rows
        .iter()
        .filter(|row| row.state == TypeId::of::<S>() && row.label == label)
        .map(|row| row.install)
        .collect();
    for install in actions {
        install(world);
    }
}

fn retire_state<S: States>(
    world: &mut World,
    mut cursor: Local<MessageCursor<StateTransitionEvent<S>>>,
) {
    let exited: Vec<_> = cursor
        .read(world.resource::<Messages<StateTransitionEvent<S>>>())
        .filter_map(|event| event.exited.clone())
        .collect();
    for state in exited {
        let label = format!("{state:?}");
        let actions: Vec<_> = world
            .resource::<ScopeRegistry>()
            .rows
            .iter()
            .rev()
            .filter(|row| row.state == TypeId::of::<S>() && row.label == label)
            .map(|row| (row.retire, row.present, row.kind, row.name))
            .collect();
        for (retire, present, kind, name) in &actions {
            retire(world);
            if matches!(kind, ScopeKind::Resource | ScopeKind::Staged) {
                assert!(!present(world), "{name} survived scope exit");
            }
        }
        world.resource_mut::<ScopeEpoch<S>>().0 =
            world.resource::<ScopeEpoch<S>>().0.wrapping_add(1);
        // Match-epoch consumers (audio voices) must also drop what a restarted
        // timeline played.
        if TypeId::of::<S>() == TypeId::of::<Timeline>() {
            world.resource_mut::<ScopeEpoch<MatchScope>>().0 =
                world.resource::<ScopeEpoch<MatchScope>>().0.wrapping_add(1);
        }
        if !actions.is_empty() {
            world.resource_mut::<crate::Retiring>().mark_teardown();
            let mut registry = world.resource_mut::<ScopeRegistry>();
            registry.retirement_serial = registry.retirement_serial.wrapping_add(1);
            registry.retired_resources = actions.len();
        }
        let mut entities = world.query_filtered::<&DespawnOnExit<S>, bevy::ecs::query::Allow<bevy::ecs::entity_disabling::Disabled>>();
        assert!(
            !entities.iter(world).any(|owner| owner.0 == state),
            "scope entity survived exit of {label}"
        );
    }
}

fn verify_registry(world: &World) {
    for row in &world.resource::<ScopeRegistry>().rows {
        if matches!(row.kind, ScopeKind::Resource | ScopeKind::Staged) {
            assert!(
                (row.active)(world, &row.label) && (row.optional || (row.present)(world))
                    || !(row.active)(world, &row.label) && !(row.present)(world),
                "{} outside {}",
                row.name,
                row.label
            );
        }
        let install_pending = matches!(
            world.resource::<NextState<MatchScope>>(),
            NextState::Pending(MatchScope::Live)
        );
        if !install_pending {
            assert!(
                (row.staging_empty)(world),
                "{} staging not consumed",
                row.name
            );
        }
    }
}

pub fn discard_staging(world: &mut World) {
    let discard: Vec<_> = world
        .resource::<ScopeRegistry>()
        .rows
        .iter()
        .map(|row| row.discard)
        .collect();
    for retire in discard {
        retire(world);
    }
}

pub trait ScopeApp {
    fn init_app<R: Resource + FromWorld>(&mut self) -> &mut Self;
    fn scoped<R: Resource + FromWorld>(&mut self, scope: impl States) -> &mut Self;
    fn staged<R: Resource>(&mut self, scope: impl States) -> &mut Self;
    fn scoped_message<M: Message>(&mut self, scope: impl States) -> &mut Self;
}

fn present<R: Resource>(world: &World) -> bool {
    world.contains_resource::<R>()
}
fn active<S: States>(world: &World, label: &str) -> bool {
    world
        .get_resource::<State<S>>()
        .is_some_and(|state| format!("{:?}", state.get()) == label)
}
fn install<R: Resource + FromWorld>(world: &mut World) {
    assert!(
        !world.contains_resource::<R>(),
        "{} already installed",
        type_name::<R>()
    );
    let value = world
        .get_resource_mut::<Staging<R>>()
        .and_then(|mut staging| staging.0.take())
        .unwrap_or_else(|| R::from_world(world));
    world.insert_resource(value);
}
fn install_staged<R: Resource>(world: &mut World) {
    let value = world
        .resource_mut::<Staging<R>>()
        .0
        .take()
        .unwrap_or_else(|| panic!("{} was not staged", type_name::<R>()));
    world.insert_resource(value);
}
fn retire<R: Resource>(world: &mut World) {
    if let Some(value) = world.remove_resource::<R>() {
        world.resource_mut::<crate::Retiring>().hand_over(value);
    }
    if let Some(mut staged) = world.get_resource_mut::<Staging<R>>()
        && let Some(value) = staged.0.take()
    {
        world.resource_mut::<crate::Retiring>().hand_over(value);
    }
}
fn retire_staging<R: Resource>(world: &mut World) {
    if let Some(mut staging) = world.get_resource_mut::<Staging<R>>()
        && let Some(value) = staging.0.take()
    {
        world.resource_mut::<crate::Retiring>().hand_over(value);
    }
}
fn staging_empty<R: Resource>(world: &World) -> bool {
    world
        .get_resource::<Staging<R>>()
        .is_none_or(Staging::is_empty)
}
fn messages_present<M: Message>(world: &World) -> bool {
    world
        .get_resource::<Messages<M>>()
        .is_some_and(|messages| !messages.is_empty())
}
fn clear_messages<M: Message>(world: &mut World) {
    world.resource_mut::<Messages<M>>().clear();
}
fn noop(_: &mut World) {}
fn empty(_: &World) -> bool {
    true
}

fn register<R: 'static, S: States>(
    app: &mut App,
    scope: S,
    kind: ScopeKind,
    install: fn(&mut World),
    retire: fn(&mut World),
    present: fn(&World) -> bool,
    staging_empty: fn(&World) -> bool,
    discard: fn(&mut World),
) {
    let label = format!("{scope:?}");
    assert!(
        (TypeId::of::<S>() == TypeId::of::<MatchScope>()
            && matches!(label.as_str(), "Loading" | "Live"))
            || (TypeId::of::<S>() == TypeId::of::<RoundPhase>() && label == "Playing")
            || (TypeId::of::<S>() == TypeId::of::<Timeline>() && label == "Running"),
        "unsupported resource scope {label}"
    );
    if !app.is_plugin_added::<ScopePlugin>() {
        app.add_plugins(ScopePlugin);
    }
    let label = format!("{scope:?}");
    let mut registry = app.world_mut().resource_mut::<ScopeRegistry>();
    if let Some(row) = registry.rows.iter().find(|row| row.id == TypeId::of::<R>()) {
        assert!(
            row.state == TypeId::of::<S>() && row.label == label && row.kind == kind,
            "{} has conflicting scope owners",
            type_name::<R>()
        );
        return;
    }
    let row = Row {
        id: TypeId::of::<R>(),
        state: TypeId::of::<S>(),
        label,
        name: type_name::<R>(),
        kind,
        optional: false,
        live: false,
        install,
        retire,
        present,
        staging_empty,
        active: active::<S>,
        discard,
    };
    if TypeId::of::<R>() == TypeId::of::<crate::WorldGeneration>() {
        registry.rows.insert(0, row);
    } else {
        registry.rows.push(row);
    }
}

impl ScopeApp for App {
    fn init_app<R: Resource + FromWorld>(&mut self) -> &mut Self {
        self.init_resource::<R>()
    }
    fn scoped<R: Resource + FromWorld>(&mut self, scope: impl States) -> &mut Self {
        register::<R, _>(
            self,
            scope,
            ScopeKind::Resource,
            install::<R>,
            retire::<R>,
            present::<R>,
            staging_empty::<R>,
            retire_staging::<R>,
        );
        self
    }
    fn staged<R: Resource>(&mut self, scope: impl States) -> &mut Self {
        register::<R, _>(
            self,
            scope,
            ScopeKind::Staged,
            install_staged::<R>,
            retire::<R>,
            present::<R>,
            staging_empty::<R>,
            retire_staging::<R>,
        );
        self.init_resource::<Staging<R>>()
    }
    fn scoped_message<M: Message>(&mut self, scope: impl States) -> &mut Self {
        register::<Messages<M>, _>(
            self,
            scope,
            ScopeKind::Message,
            noop,
            clear_messages::<M>,
            messages_present::<M>,
            empty,
            noop,
        );
        self.add_message::<M>()
    }
}

pub fn stage<R: Resource>(world: &mut World, value: R) {
    let registered = world
        .resource::<ScopeRegistry>()
        .rows
        .iter()
        .find(|row| row.id == TypeId::of::<R>())
        .map(|row| {
            assert!(
                row.state == TypeId::of::<MatchScope>() && row.label == "Live",
                "{} staged for a conflicting scope",
                type_name::<R>()
            );
        })
        .is_some();
    if !registered {
        world.resource_mut::<ScopeRegistry>().rows.push(Row {
            id: TypeId::of::<R>(),
            state: TypeId::of::<MatchScope>(),
            label: format!("{:?}", MatchScope::Live),
            name: type_name::<R>(),
            kind: ScopeKind::Staged,
            optional: true,
            live: false,
            install: install_optional_staged::<R>,
            retire: retire::<R>,
            present: present::<R>,
            staging_empty: staging_empty::<R>,
            active: active::<MatchScope>,
            discard: retire_staging::<R>,
        });
    }
    world.init_resource::<Staging<R>>();
    world.resource_mut::<Staging<R>>().put(value);
}

fn install_optional_staged<R: Resource>(world: &mut World) {
    if let Some(value) = world.resource_mut::<Staging<R>>().0.take() {
        world.insert_resource(value);
    }
}

pub fn insert<R: Resource>(world: &mut World, value: R, scope: MatchScope) {
    assert_eq!(
        *world.resource::<State<MatchScope>>().get(),
        scope,
        "{} inserted outside its scope",
        type_name::<R>()
    );
    let registered = world
        .resource::<ScopeRegistry>()
        .rows
        .iter()
        .find(|row| row.id == TypeId::of::<R>())
        .map(|row| {
            assert!(
                row.state == TypeId::of::<MatchScope>() && row.label == format!("{scope:?}"),
                "{} inserted for a conflicting scope",
                type_name::<R>()
            );
        })
        .is_some();
    if !registered {
        world.resource_mut::<ScopeRegistry>().rows.push(Row {
            id: TypeId::of::<R>(),
            state: TypeId::of::<MatchScope>(),
            label: format!("{scope:?}"),
            name: type_name::<R>(),
            kind: ScopeKind::Resource,
            optional: true,
            live: false,
            install: noop,
            retire: retire::<R>,
            present: present::<R>,
            staging_empty: staging_empty::<R>,
            active: active::<MatchScope>,
            discard: retire_staging::<R>,
        });
    }
    if let Some(previous) = world.remove_resource::<R>() {
        world.resource_mut::<crate::Retiring>().hand_over(previous);
    }
    world.insert_resource(value);
}

fn capture_census(world: &mut World) {
    let live: Vec<_> = world
        .resource::<ScopeRegistry>()
        .rows
        .iter()
        .map(|row| (row.present)(world))
        .collect();
    for (row, live) in world
        .resource_mut::<ScopeRegistry>()
        .rows
        .iter_mut()
        .zip(live)
    {
        row.live = live;
    }
}
