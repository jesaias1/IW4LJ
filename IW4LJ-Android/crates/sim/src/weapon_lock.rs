#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WeaponLock {
    pub weapon: u32,
    pub life: u32,
    pub flags: u8,
    pub target: [f32; 3],
    pub aim: Option<crate::MissileTarget>,
    pub acquire_started_at: i32,
}

impl WeaponLock {
    pub fn locking(self) -> bool {
        self.flags & 3 == 1
    }
    pub fn locked(self) -> bool {
        self.flags & 2 != 0
    }
    pub fn too_close(self) -> bool {
        self.flags & 16 != 0
    }
    pub fn can_fire(self, weapon: u32, life: u32) -> bool {
        self.weapon == weapon
            && self.life == life
            && self.locked()
            && self.flags & 48 == 0
            && self.aim.is_some()
    }
}
