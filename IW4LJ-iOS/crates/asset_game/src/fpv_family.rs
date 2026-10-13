use crate::FpvMeshIndex;
use asset_core::{Family, FamilyId, Iw4, Iw5, T5, T6};
use asset_model::{FamilyFpvMesh, FpvMeshCatalog};

#[derive(Clone, Copy, Debug)]
pub struct SoldierFpvConnection<G: Family, H: Family> {
    gun: FamilyFpvMesh<G>,
    hands: FamilyFpvMesh<H>,
}

impl<G: Family, H: Family> SoldierFpvConnection<G, H> {
    pub fn connect(gun: FamilyFpvMesh<G>, hands: FamilyFpvMesh<H>) -> Option<Self> {
        (gun.owner() == hands.owner()).then_some(Self { gun, hands })
    }
}

pub type NativeFpvConnection<F> = SoldierFpvConnection<F, F>;

#[derive(Clone, Copy, Debug)]
pub enum SoldierHandsConnection<G: Family> {
    Iw4(SoldierFpvConnection<G, Iw4>),
    Iw5(SoldierFpvConnection<G, Iw5>),
    T5(SoldierFpvConnection<G, T5>),
    T6(SoldierFpvConnection<G, T6>),
}

impl<G: Family> SoldierHandsConnection<G> {
    fn bind(catalog: &FpvMeshCatalog, gun: FpvMeshIndex, hands: FpvMeshIndex) -> Option<Self> {
        let gun = catalog.family_mesh::<G>(gun)?;
        match catalog.get_at(hands.order())?.namespace {
            FamilyId::Iw4 => {
                SoldierFpvConnection::connect(gun, catalog.family_mesh(hands)?).map(Self::Iw4)
            }
            FamilyId::Iw5 => {
                SoldierFpvConnection::connect(gun, catalog.family_mesh(hands)?).map(Self::Iw5)
            }
            FamilyId::T5 => {
                SoldierFpvConnection::connect(gun, catalog.family_mesh(hands)?).map(Self::T5)
            }
            FamilyId::T6 => {
                SoldierFpvConnection::connect(gun, catalog.family_mesh(hands)?).map(Self::T6)
            }
        }
    }

    fn gun(self) -> FpvMeshIndex {
        match self {
            Self::Iw4(c) => c.gun.index(),
            Self::Iw5(c) => c.gun.index(),
            Self::T5(c) => c.gun.index(),
            Self::T6(c) => c.gun.index(),
        }
    }

    fn hands(self) -> FpvMeshIndex {
        match self {
            Self::Iw4(c) => c.hands.index(),
            Self::Iw5(c) => c.hands.index(),
            Self::T5(c) => c.hands.index(),
            Self::T6(c) => c.hands.index(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FpvFamilyConnection {
    Iw4(SoldierHandsConnection<Iw4>),
    Iw5(SoldierHandsConnection<Iw5>),
    T5(SoldierHandsConnection<T5>),
    T6(SoldierHandsConnection<T6>),
}

impl FpvFamilyConnection {
    pub(crate) fn bind(
        catalog: &FpvMeshCatalog,
        gun: FpvMeshIndex,
        hands: FpvMeshIndex,
    ) -> Option<Self> {
        match catalog.get_at(gun.order())?.namespace {
            FamilyId::Iw4 => SoldierHandsConnection::bind(catalog, gun, hands).map(Self::Iw4),
            FamilyId::Iw5 => SoldierHandsConnection::bind(catalog, gun, hands).map(Self::Iw5),
            FamilyId::T5 => SoldierHandsConnection::bind(catalog, gun, hands).map(Self::T5),
            FamilyId::T6 => SoldierHandsConnection::bind(catalog, gun, hands).map(Self::T6),
        }
    }

    pub fn family(self) -> FamilyId {
        match self {
            Self::Iw4(_) => FamilyId::Iw4,
            Self::Iw5(_) => FamilyId::Iw5,
            Self::T5(_) => FamilyId::T5,
            Self::T6(_) => FamilyId::T6,
        }
    }

    pub fn gun(self) -> FpvMeshIndex {
        match self {
            Self::Iw4(c) => c.gun(),
            Self::Iw5(c) => c.gun(),
            Self::T5(c) => c.gun(),
            Self::T6(c) => c.gun(),
        }
    }

    pub fn hands(self) -> FpvMeshIndex {
        match self {
            Self::Iw4(c) => c.hands(),
            Self::Iw5(c) => c.hands(),
            Self::T5(c) => c.hands(),
            Self::T6(c) => c.hands(),
        }
    }
}
