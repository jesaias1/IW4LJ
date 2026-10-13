use bevy::prelude::*;
use frame::ScopeApp;

pub struct RenderFxPlugin;

impl Plugin for RenderFxPlugin {
    fn build(&self, app: &mut App) {
        app.scoped::<crate::FxCodeMeshPlan>(frame::MatchScope::Live)
            .scoped::<crate::FxParticleCloudPlan>(frame::MatchScope::Live)
            .scoped::<crate::GfxMarkMeshPlan>(frame::MatchScope::Live)
            .scoped::<crate::FxJournalCursor>(frame::MatchScope::Live)
            .scoped::<crate::CombatFxDump>(frame::MatchScope::Live)
            .scoped::<crate::PreparedFxElemInfos>(frame::MatchScope::Live)
            .scoped::<crate::FxCameraOrigin>(frame::MatchScope::Live)
            .scoped::<crate::HostFxSystem>(frame::MatchScope::Live)
            .init_resource::<crate::FxMarkDvars>()
            .init_resource::<crate::LaserDvars>()
            .scoped::<crate::HostFxDlights>(frame::MatchScope::Live)
            .scoped::<crate::HostFxPostLights>(frame::MatchScope::Live)
            .scoped::<crate::FxWorldColorImages>(frame::MatchScope::Live)
            .scoped::<crate::FxDumpRequest>(frame::MatchScope::Live)
            .scoped::<crate::PresentedVehicleFx>(frame::MatchScope::Live)
            .scoped::<crate::TracerDrawGate>(frame::MatchScope::Live)
            .scoped::<crate::TracerWorld>(frame::MatchScope::Live)
            .scoped::<crate::EntityMarks>(frame::MatchScope::Live)
            .scoped::<crate::FxModelDrawPlan>(frame::MatchScope::Live);
        crate::system::register_fx_orchestration(app);
    }
}
