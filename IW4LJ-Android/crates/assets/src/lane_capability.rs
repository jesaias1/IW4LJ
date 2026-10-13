use crate::lane::lane;
use asset_core::ZoneGame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaneStatus {
    SupportedPopulated,
    SupportedEmpty,
    MissingDecoder,
    MissingEvidence,
    UnsupportedByRuntimeProfile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparedCapability {
    Envelope,
    PreparedWorld,
    CollisionSpawns,
    WeaponCatalog,

    BodySkeleton,
    PlayableFfa,
}

pub fn lane_status(game: ZoneGame, capability: PreparedCapability) -> LaneStatus {
    for &(cap, status) in lane(game).capabilities() {
        if cap == capability {
            return status;
        }
    }

    LaneStatus::MissingEvidence
}
