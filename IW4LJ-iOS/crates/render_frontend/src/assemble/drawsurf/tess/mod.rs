pub mod glass;
pub mod smodel;
pub mod world;
pub mod xmodel;

pub use render_fx::drawsurf::tess::{fx, mark, particle_cloud};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TessKind {
    World,

    Smodel,

    XModel,

    CodeMesh,

    ParticleCloud,

    MarkMesh,

    Glass,
}

pub use glass::{GfxGlassMeshPlan, GlassTable};
pub use render_fx::drawsurf::tess::mark::GfxMarkSubKey;
pub use render_fx::drawsurf::tess::mark::mark_mesh_surface_samplers;
pub use render_fx::{FxCodeMeshPlan, FxParticleCloudPlan, GfxMarkMeshPlan};
pub use smodel::{PackedVertexRefusal, SmodelGpuPlan, SmodelPassMaterial, SmodelVertex};
pub use world::{WorldDrawGpuPlan, WorldPassMaterial, WorldVertex, WorldVertexRefusal};
pub use xmodel::{
    DynEntDrawPlan, FpvDrawPlan, FxModelDrawPlan, ItemDrawPlan, MissileDrawPlan,
    RemoteBodyDrawPlan, ScriptModelDrawPlan, XModelDrawPlan,
};

use std::sync::Arc;

pub(crate) const SHARE_BANKS: usize = 3;

#[derive(Clone, Debug)]
pub(crate) struct ShareBanks<T> {
    banks: [Arc<Vec<T>>; SHARE_BANKS],
    write: usize,
}

impl<T> Default for ShareBanks<T> {
    fn default() -> Self {
        Self {
            banks: [
                Arc::new(Vec::new()),
                Arc::new(Vec::new()),
                Arc::new(Vec::new()),
            ],
            write: 0,
        }
    }
}

impl<T> ShareBanks<T> {
    fn acquire_unique(&mut self) -> &mut Vec<T> {
        for i in 0..SHARE_BANKS {
            let idx = (self.write + i) % SHARE_BANKS;
            if Arc::strong_count(&self.banks[idx]) == 1 {
                self.write = idx;
                return Arc::get_mut(&mut self.banks[idx]).expect("unique bank");
            }
        }
        self.write = (self.write + 1) % SHARE_BANKS;
        self.banks[self.write] = Arc::new(Vec::new());
        Arc::get_mut(&mut self.banks[self.write]).expect("new bank is unique")
    }

    pub(crate) fn published(&self) -> Arc<Vec<T>> {
        Arc::clone(&self.banks[self.write])
    }

    pub(crate) fn take_write(&mut self) -> Vec<T> {
        let mut rows = std::mem::take(self.acquire_unique());
        rows.clear();
        rows
    }

    pub(crate) fn put_write(&mut self, rows: Vec<T>) {
        let vec =
            Arc::get_mut(&mut self.banks[self.write]).expect("write bank unique after take_write");
        *vec = rows;
    }
}

pub(crate) fn reset_rows<T>(rows: &mut Arc<Vec<T>>) {
    match Arc::get_mut(rows) {
        Some(v) => v.clear(),
        None => *rows = Arc::new(Vec::new()),
    }
}

pub(crate) fn published_rows<T>(share: &Option<Arc<Vec<T>>>) -> &[T] {
    share.as_ref().map(|rows| rows.as_slice()).unwrap_or(&[])
}

pub(crate) fn publish_index_ranges(
    draws: impl IntoIterator<Item = (u32, u32)>,
) -> Arc<Vec<(u32, u32)>> {
    Arc::new(draws.into_iter().collect())
}

pub mod sky;
