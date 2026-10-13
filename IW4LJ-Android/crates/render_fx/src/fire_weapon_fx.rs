use entity_iw4::bullet_hit_event;
use fx_iw4::impact_table_row;

pub fn fire_weapon_fx_should_client_trace(impact_type: i32) -> bool {
    impact_table_row(impact_type, false).is_some() && bullet_hit_event(impact_type, false).is_none()
}
