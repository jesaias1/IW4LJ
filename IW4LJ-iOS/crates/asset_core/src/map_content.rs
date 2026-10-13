use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlassPaneBasis {
    pub origin: [f32; 3],
    pub axis_s: [f32; 3],
    pub axis_t: [f32; 3],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MapContentDefinition {
    pub glass: GlassContent,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum GlassContent {
    Absent,
    #[default]
    Unsupported,
    Invalid(String),
    Prepared(GlassDefinition),
}

#[derive(Clone, Debug, PartialEq)]
pub struct GlassDefinition {
    panes: Vec<GlassPaneBasis>,
    names: BTreeMap<String, Vec<u32>>,
    collision_pieces: BTreeSet<u32>,
}

impl GlassDefinition {
    pub fn checked(
        panes: Vec<GlassPaneBasis>,
        names: Vec<(String, Vec<u32>)>,
        collision_pieces: impl IntoIterator<Item = u32>,
    ) -> Result<Self, String> {
        for (id, pane) in panes.iter().enumerate() {
            if !pane
                .origin
                .iter()
                .chain(&pane.axis_s)
                .chain(&pane.axis_t)
                .all(|value| value.is_finite())
            {
                return Err(format!("glass piece {id} has non-finite geometry"));
            }
            let norm = |axis: [f32; 3]| axis.iter().map(|v| v * v).sum::<f32>();
            if norm(pane.axis_s) < 0.5 || norm(pane.axis_t) < 0.5 {
                return Err(format!("glass piece {id} has a degenerate plane"));
            }
        }
        let collision_pieces: BTreeSet<_> = collision_pieces.into_iter().collect();
        for id in &collision_pieces {
            if *id as usize >= panes.len() {
                return Err(format!("glass collision piece {id} has no visual plane"));
            }
        }
        let mut named = BTreeMap::new();
        for (name, pieces) in names {
            if name.is_empty() || named.contains_key(&name) {
                return Err(format!("invalid or duplicate glass name {name:?}"));
            }
            let mut seen = BTreeSet::new();
            for id in &pieces {
                if *id as usize >= panes.len() || !seen.insert(*id) {
                    return Err(format!("glass set {name:?} has invalid piece {id}"));
                }
            }
            named.insert(name, pieces);
        }
        Ok(Self {
            panes,
            names: named,
            collision_pieces,
        })
    }

    pub fn panes(&self) -> &[GlassPaneBasis] {
        &self.panes
    }
    pub fn named(&self, name: &str) -> &[u32] {
        self.names.get(name).map_or(&[], Vec::as_slice)
    }
    pub fn names(&self) -> &BTreeMap<String, Vec<u32>> {
        &self.names
    }
    pub fn collision_pieces(&self) -> &BTreeSet<u32> {
        &self.collision_pieces
    }
}
