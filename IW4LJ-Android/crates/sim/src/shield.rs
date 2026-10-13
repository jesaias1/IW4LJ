#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShieldAttachment {
    pub weapon: u32,
    pub on_back: bool,
}

impl ShieldAttachment {
    pub const fn tag(self) -> &'static str {
        if self.on_back {
            "tag_shield_back"
        } else {
            "tag_weapon_left"
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShieldCarrierCollision {
    pub normal: [f32; 3],
    pub bones: Vec<xmodel_runtime::CollisionBone>,
}

pub(crate) const HITLOC: u8 = 19;
pub(crate) const SURFACE: u8 = 29;

pub(crate) fn ricochet_direction(incoming: [f32; 3], normal: [f32; 3]) -> Option<[f32; 3]> {
    let incoming = glam::Vec3::from_array(incoming).try_normalize()?;
    let mut normal = glam::Vec3::from_array(normal).try_normalize()?;
    if incoming.dot(normal) > 0.0 {
        return None;
    }
    if normal.z < 0.0 {
        normal.z = (normal.z + 0.5).min(0.1);
        normal = normal.try_normalize()?;
    }
    let outward = -incoming;
    let result = if (outward.dot(normal) - 1.0).abs() < 0.001 {
        normal
    } else {
        outward.cross(normal.cross(outward)).try_normalize()?
    };
    Some(result.to_array())
}
