#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScopeZoom {
    fovs: [f32; 3],
    count: u8,
}

impl ScopeZoom {
    pub const NONE: Self = Self {
        fovs: [0.0; 3],
        count: 0,
    };
    pub fn from_fovs(fovs: [f32; 3]) -> Self {
        let mut zoom = Self::default();
        for fov in fovs {
            if fov.is_finite()
                && fov > 1.0
                && fov < 180.0
                && !zoom.fovs[..usize::from(zoom.count)].contains(&fov)
            {
                zoom.fovs[usize::from(zoom.count)] = fov;
                zoom.count += 1;
            }
        }
        zoom
    }

    pub fn levels(self) -> u8 {
        self.count
    }

    pub fn is_variable(self) -> bool {
        self.count > 1
    }

    pub fn fov(self, selected: u32) -> Option<f32> {
        (self.count != 0).then(|| self.fovs[(selected % u32::from(self.count)) as usize])
    }
}

pub fn update_scope_zoom(
    ps: &mut playerstate_iw4::PlayerState,
    buttons: u32,
    old_buttons: u32,
    zoom: ScopeZoom,
) {
    use playerstate_iw4::{buttons as bits, pm_flags, weap_flags};
    if !zoom.is_variable() {
        ps.scope_zoom_level = 0;
        return;
    }
    ps.scope_zoom_level %= u32::from(zoom.levels());
    if buttons & bits::CHANGE_ZOOM != 0
        && old_buttons & bits::CHANGE_ZOOM == 0
        && ps.f_weapon_pos_frac == 1.0
        && ps.health > 0
        && ps.pm_flags & (pm_flags::SPRINTING | pm_flags::MANTLE | pm_flags::LADDER) == 0
        && ps.weap_flags & weap_flags::OFFHAND_VIEW == 0
    {
        ps.scope_zoom_level = (ps.scope_zoom_level + 1) % u32::from(zoom.levels());
    }
}
