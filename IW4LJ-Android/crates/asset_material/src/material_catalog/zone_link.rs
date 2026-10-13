use super::definitions::{OwnedTechnique, TechniqueSetFacts};
use fastfile_iw4::{Ptr, block_is_aliasable};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub(super) enum LinkKind {
    Material,
    Image,
    Techset,
    VertexShader,
    PixelShader,
    VertexDecl,
}

#[derive(Clone, Copy, Debug)]
enum Link {
    Direct(usize),
    Alias(Ptr),
}

#[derive(Clone, Debug, Default)]
pub(super) struct ZoneLinkState {
    materials: HashMap<Ptr, Link>,
    images: HashMap<Ptr, Link>,
    techsets: HashMap<Ptr, Link>,
    vertex_shaders: HashMap<Ptr, Link>,
    pixel_shaders: HashMap<Ptr, Link>,
    vertex_decls: HashMap<Ptr, Link>,
    technique_bodies: HashMap<Ptr, OwnedTechnique>,
    last_technique_set: Option<TechniqueSetFacts>,
}

impl ZoneLinkState {
    pub(super) fn begin_zone(&mut self) {
        *self = Self::default();
    }

    fn map(&self, kind: LinkKind) -> &HashMap<Ptr, Link> {
        match kind {
            LinkKind::Material => &self.materials,
            LinkKind::Image => &self.images,
            LinkKind::Techset => &self.techsets,
            LinkKind::VertexShader => &self.vertex_shaders,
            LinkKind::PixelShader => &self.pixel_shaders,
            LinkKind::VertexDecl => &self.vertex_decls,
        }
    }

    fn map_mut(&mut self, kind: LinkKind) -> &mut HashMap<Ptr, Link> {
        match kind {
            LinkKind::Material => &mut self.materials,
            LinkKind::Image => &mut self.images,
            LinkKind::Techset => &mut self.techsets,
            LinkKind::VertexShader => &mut self.vertex_shaders,
            LinkKind::PixelShader => &mut self.pixel_shaders,
            LinkKind::VertexDecl => &mut self.vertex_decls,
        }
    }

    pub(super) fn resolve(&self, kind: LinkKind, mut slot: Ptr) -> Option<usize> {
        for _ in 0..32 {
            match self.map(kind).get(&slot).copied()? {
                Link::Direct(index) => return Some(index),
                Link::Alias(target) => slot = target,
            }
        }
        None
    }

    fn bind(&mut self, kind: LinkKind, slot: Ptr, link: Link) {
        if block_is_aliasable(slot.block) {
            self.map_mut(kind).insert(slot, link);
        }
    }

    pub(super) fn bind_direct(&mut self, kind: LinkKind, slot: Ptr, index: usize) {
        self.bind(kind, slot, Link::Direct(index));
    }

    pub(super) fn bind_alias(&mut self, kind: LinkKind, slot: Ptr, target: Ptr) {
        self.bind(kind, slot, Link::Alias(target));
    }

    pub(super) fn select_techset(&mut self, facts: Option<TechniqueSetFacts>) {
        self.last_technique_set = facts;
    }

    pub(super) fn take_techset(&mut self) -> Option<TechniqueSetFacts> {
        self.last_technique_set.take()
    }

    pub(super) fn technique(&self, body: Ptr) -> Option<OwnedTechnique> {
        self.technique_bodies.get(&body).cloned()
    }

    pub(super) fn remember_technique(&mut self, body: Ptr, technique: OwnedTechnique) {
        self.technique_bodies.insert(body, technique);
    }
}
