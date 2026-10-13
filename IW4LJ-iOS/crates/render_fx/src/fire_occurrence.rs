use std::collections::HashMap;
use std::time::Instant;

use bevy::prelude::Resource;

const CAPACITY: usize = 8192;
const RETENTION_MS: i32 = 5000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FireFxOccurrence {
    Muzzle,
    Brass,
    Tracer { pellet: u16, segment: u16 },
    Impact { pellet: u16, segment: u16 },
    Glass { pellet: u16, segment: u16 },
    Marks { pellet: u16, segment: u16 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FireFxResult {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub normal: [f32; 3],
    pub surface: u8,
    pub flags: u32,
    pub target: Option<u16>,
    pub flesh: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireFxOutcome {
    Created(u32),
    Scheduled(u32),
    Applied(u32),
    NotApplicable(&'static str),
    PolicySkipped(&'static str),
    DeferredUntilAuthority,
    DependencyRefused(&'static str),
    RetryableFailure(fx::SpawnFail),
    Duplicate,
    StaleScope,
    RefusedFire,
    HistoryCapacityRefused,
    AuthorityConflict,
}

impl FireFxOutcome {
    fn completed(self) -> bool {
        matches!(
            self,
            Self::Created(_) | Self::Scheduled(_) | Self::Applied(_) | Self::PolicySkipped(_)
        )
    }
}

#[derive(Clone, Copy)]
pub struct FireFxRequest {
    pub world: frame::WorldStamp,
    pub timeline: u64,
    pub domain: net::EntityEventDomain,
    pub cause: Option<sim::FireCause>,
    pub occurrence: FireFxOccurrence,
    pub result: Option<FireFxResult>,
    pub now: i32,
}

impl FireFxRequest {
    pub fn event(
        event: &net::DispatchedEntityEvent,
        occurrence: FireFxOccurrence,
        now: i32,
    ) -> Self {
        Self {
            world: event.world,
            timeline: event.timeline,
            domain: event.domain,
            cause: event.payload.fire_cause,
            occurrence,
            result: None,
            now,
        }
    }
}

struct Receipt {
    at: i32,
    outcome: Option<FireFxOutcome>,
    result: Option<FireFxResult>,
    authoritative: bool,
}

#[derive(Clone, Debug, Default)]
pub struct FireFxBudgetStats {
    pub muzzle: u64,
    pub brass: u64,
    pub tracer: u64,
    pub impact: u64,
    pub glass: u64,
    pub marks: u64,
}

#[derive(Clone, Debug, Default)]
pub struct FireFxStats {
    pub world: Option<u64>,
    pub timeline: u64,
    pub now: i32,
    pub attempts: u64,
    pub created: u64,
    pub applied: u64,
    pub scheduled: u64,
    pub duplicates: u64,
    pub deferred: u64,
    pub dependency_refused: u64,
    pub retryable_refused: u64,
    pub budget_refusals: FireFxBudgetStats,
    pub capacity_refused: u64,
    pub corrections: u64,
    pub authority_conflicts: u64,
    pub maintenance_sweeps: u64,
    pub maintenance_visited: u64,
    pub expired: u64,
    pub maintenance_nanos: u64,
    pub history_len: usize,
    pub history_capacity: usize,
}

#[derive(Resource, Default)]
pub struct PresentedFireFx {
    scope: Option<(Option<u64>, u64)>,
    receipts: HashMap<(sim::FireCause, FireFxOccurrence), Receipt>,
    last_now: Option<i32>,
    pub stats: FireFxStats,
}

impl PresentedFireFx {
    fn adopt_scope(&mut self, generation: frame::WorldStamp, timeline: u64, now: i32) -> bool {
        let scope = (generation.0, timeline);
        let changed = self.scope != Some(scope)
            || self
                .last_now
                .is_some_and(|previous| now.wrapping_sub(previous) < 0);
        if changed {
            self.scope = Some(scope);
            self.receipts.clear();
        }
        self.stats.world = generation.0;
        self.stats.timeline = timeline;
        self.stats.now = now;
        self.last_now = Some(now);
        changed
    }

    pub fn maintain(&mut self, generation: frame::WorldStamp, timeline: u64, now: i32) -> bool {
        let started = Instant::now();
        let changed = self.adopt_scope(generation, timeline, now);
        self.stats.maintenance_sweeps += 1;
        self.stats.maintenance_visited += self.receipts.len() as u64;
        let before = self.receipts.len();
        self.receipts
            .retain(|_, receipt| (0..RETENTION_MS).contains(&now.wrapping_sub(receipt.at)));
        self.stats.expired += (before - self.receipts.len()) as u64;
        self.stats.maintenance_nanos +=
            started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
        self.stats.history_len = self.receipts.len();
        self.stats.history_capacity = self.receipts.capacity();
        changed
    }

    pub fn execute(
        &mut self,
        generation: frame::WorldStamp,
        timeline: u64,
        request: FireFxRequest,
        verdicts: &net::FireVerdictState,
        play: impl FnOnce() -> FireFxOutcome,
    ) -> FireFxOutcome {
        self.stats.attempts += 1;
        if generation != request.world || timeline != request.timeline {
            return FireFxOutcome::StaleScope;
        }
        if request.cause.is_some_and(|cause| {
            verdicts.status(generation, cause) == net::PredictedFireStatus::Refused
        }) {
            return FireFxOutcome::RefusedFire;
        }
        self.adopt_scope(generation, timeline, request.now);
        let key = request.cause.map(|cause| (cause, request.occurrence));
        let authoritative = request.domain != net::EntityEventDomain::Predicted;
        if let Some(key) = key {
            if self.receipts.get(&key).is_some_and(|receipt| {
                !(0..RETENTION_MS).contains(&request.now.wrapping_sub(receipt.at))
            }) {
                self.receipts.remove(&key);
                self.stats.expired += 1;
            }
            if let Some(receipt) = self.receipts.get_mut(&key) {
                if authoritative && request.result.is_some() {
                    if receipt.authoritative && receipt.result != request.result {
                        self.stats.authority_conflicts += 1;
                        return FireFxOutcome::AuthorityConflict;
                    }
                    if !receipt.authoritative && receipt.result != request.result {
                        self.stats.corrections += 1;
                    }
                    receipt.result = request.result;
                    receipt.authoritative = true;
                }
                if receipt.outcome.is_some() || (receipt.authoritative && !authoritative) {
                    self.stats.duplicates += 1;
                    return FireFxOutcome::Duplicate;
                }
            } else if self.receipts.len() >= CAPACITY {
                self.stats.capacity_refused += 1;
                return FireFxOutcome::HistoryCapacityRefused;
            }
        }
        let outcome = play();
        match outcome {
            FireFxOutcome::Created(_) => self.stats.created += 1,
            FireFxOutcome::Scheduled(_) => self.stats.scheduled += 1,
            FireFxOutcome::Applied(_) => self.stats.applied += 1,
            FireFxOutcome::DeferredUntilAuthority => self.stats.deferred += 1,
            FireFxOutcome::DependencyRefused(_) => self.stats.dependency_refused += 1,
            FireFxOutcome::RetryableFailure(_) => {
                self.stats.retryable_refused += 1;
                let count = match request.occurrence {
                    FireFxOccurrence::Muzzle => &mut self.stats.budget_refusals.muzzle,
                    FireFxOccurrence::Brass => &mut self.stats.budget_refusals.brass,
                    FireFxOccurrence::Tracer { .. } => &mut self.stats.budget_refusals.tracer,
                    FireFxOccurrence::Impact { .. } => &mut self.stats.budget_refusals.impact,
                    FireFxOccurrence::Glass { .. } => &mut self.stats.budget_refusals.glass,
                    FireFxOccurrence::Marks { .. } => &mut self.stats.budget_refusals.marks,
                };
                *count += 1;
            }
            FireFxOutcome::HistoryCapacityRefused => self.stats.capacity_refused += 1,
            _ => {}
        }
        if let Some(key) = key {
            let receipt = self.receipts.entry(key).or_insert(Receipt {
                at: request.now,
                outcome: None,
                result: request.result,
                authoritative,
            });
            if authoritative {
                receipt.authoritative = true;
                receipt.result = request.result;
            }
            if outcome.completed() {
                receipt.outcome = Some(outcome);
                receipt.at = request.now;
            }
        }
        self.stats.history_len = self.receipts.len();
        self.stats.history_capacity = self.receipts.capacity();
        outcome
    }
}
