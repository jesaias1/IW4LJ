use std::path::Path;

use bevy::prelude::Resource;

use crate::CapturedStringTable;

pub const FACTION_ICON_COL: i32 = 5;

const FACTION_VOICE_PREFIX_COL: i32 = 7;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArenaCharsets {
    pub map: String,
    pub allieschar: Option<String>,
    pub axischar: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MapTeamSettings {
    pub allies_color: Option<[f32; 3]>,
    pub axis_color: Option<[f32; 3]>,
    pub allies_name: Option<asset_core::AssetKey>,
    pub axis_name: Option<asset_core::AssetKey>,
    pub attackers: Option<String>,
    pub defenders: Option<String>,
    pub allies: Option<asset_core::AssetKey>,
    pub axis: Option<asset_core::AssetKey>,
    pub allies_charset: Option<String>,
    pub axis_charset: Option<String>,
    pub allies_strings: FactionStrings,
    pub axis_strings: FactionStrings,
    pub allies_voice: Option<String>,
    pub axis_voice: Option<String>,
    pub allies_music: TeamMusic,
    pub axis_music: TeamMusic,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TeamMusic {
    pub spawn: Option<String>,
    pub victory: Option<String>,
    pub defeat: Option<String>,
    pub winning: Option<String>,
    pub losing: Option<String>,
}

impl TeamMusic {
    fn prefixed(prefix: &str) -> Self {
        let music = |line: &str| Some(format!("{prefix}{line}_music"));
        Self {
            spawn: music("spawn"),
            victory: music("victory"),
            defeat: music("defeat"),
            winning: music("winning"),
            losing: music("losing"),
        }
    }

    fn t5(script: &str, team: &str) -> Self {
        let spawn =
            crate::game_nested_string_assignment(script, &["music", &format!("spawn_{team}")]);
        Self {
            spawn: spawn.map(|state| format!("mus_{}", state.to_ascii_lowercase())),
            victory: Some("mus_victory".to_owned()),
            defeat: Some("mus_loss".to_owned()),
            winning: Some("mus_time_running_out".to_owned()),
            losing: Some("mus_time_running_out".to_owned()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FactionStrings {
    pub name: Option<String>,
    pub eliminated: Option<String>,
    pub forfeited: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObjectiveVisuals {
    pub neutral_flag: Option<String>,
    pub bomb: Option<String>,
    pub bomb_explosion_fx: Option<String>,
    pub plant_weapon: Option<String>,
    pub defuse_weapon: Option<String>,
    pub flag: [Option<String>; 2],
    pub flag_carry: [Option<String>; 2],
    pub flag_base_fx: [Option<String>; 2],
    pub crate_model: [Option<String>; 2],
    pub crate_overlay: [Option<String>; 2],
}

impl ObjectiveVisuals {
    #[must_use]
    pub fn from_faction_table(
        table: &CapturedStringTable,
        allieschar: Option<&str>,
        axischar: Option<&str>,
    ) -> Self {
        let cell = |charset: Option<&str>, column| {
            let charset = charset.filter(|s| !s.is_empty())?;
            let value = table.lookup_col(charset, column);
            (!value.is_empty()).then(|| value.to_owned())
        };
        let pair = |column| [cell(allieschar, column), cell(axischar, column)];
        Self {
            neutral_flag: Some("prop_flag_neutral".to_owned()),
            bomb: Some("prop_suitcase_bomb".to_owned()),
            plant_weapon: Some("briefcase_bomb_mp".to_owned()),
            defuse_weapon: Some("briefcase_bomb_defuse_mp".to_owned()),
            bomb_explosion_fx: Some("explosions/tanker_explosion".to_owned()),
            flag: pair(10),
            flag_carry: pair(11),
            flag_base_fx: pair(13),
            crate_model: pair(18),
            crate_overlay: [
                Some("com_plasticcase_friendly".to_owned()),
                Some("com_plasticcase_enemy".to_owned()),
            ],
        }
    }

    #[must_use]
    pub fn t5(allies: Option<&str>, axis: Option<&str>) -> Self {
        let allies_flag = match allies {
            Some("marines") => "mp_flag_allies_1",
            Some("rebels") => "mp_flag_allies_3",
            _ => "mp_flag_allies_2",
        };
        let axis_flag = match axis {
            Some("russian") => "mp_flag_axis_1",
            Some("tropas") => "mp_flag_axis_3",
            _ => "mp_flag_axis_2",
        };
        let crates = [
            Some("mp_supplydrop_ally".to_owned()),
            Some("mp_supplydrop_axis".to_owned()),
        ];
        Self {
            neutral_flag: Some("mp_flag_neutral".to_owned()),
            bomb: Some("prop_suitcase_bomb".to_owned()),
            plant_weapon: Some("briefcase_bomb_mp".to_owned()),
            defuse_weapon: Some("briefcase_bomb_defuse_mp".to_owned()),
            bomb_explosion_fx: Some("maps/mp_maps/fx_mp_exp_bomb".to_owned()),
            flag: [Some(allies_flag.to_owned()), Some(axis_flag.to_owned())],
            flag_carry: [
                Some(format!("{allies_flag}_carry")),
                Some(format!("{axis_flag}_carry")),
            ],
            flag_base_fx: std::array::from_fn(|_| Some("misc/fx_ui_flagbase_gold_t5".to_owned())),
            crate_model: crates.clone(),
            crate_overlay: crates,
        }
    }

    #[must_use]
    pub fn t6(allies: Option<&str>, axis: Option<&str>) -> Self {
        let flag = |faction| match faction {
            Some("seals") => Some("mp_flag_allies_1"),
            Some("fbi") => Some("mp_flag_allies_2"),
            Some("isa") => Some("mp_flag_allies_3"),
            Some("pla") => Some("mp_flag_axis_1"),
            Some("pmc") => Some("mp_flag_axis_2"),
            Some("cd") => Some("mp_flag_axis_3"),
            _ => None,
        };
        let flags = [flag(allies), flag(axis)];
        Self {
            neutral_flag: Some("mp_flag_neutral".into()),
            bomb: Some("prop_suitcase_bomb".into()),
            bomb_explosion_fx: Some("maps/mp_maps/fx_mp_exp_bomb".into()),
            plant_weapon: Some("briefcase_bomb_mp".into()),
            defuse_weapon: Some("briefcase_bomb_defuse_mp".into()),
            flag: flags.map(|name| name.map(str::to_owned)),
            flag_carry: flags.map(|name| name.map(|name| format!("{name}_carry"))),
            crate_model: [
                Some("t6_wpn_supply_drop_ally".into()),
                Some("t6_wpn_supply_drop_axis".into()),
            ],
            crate_overlay: [
                Some("t6_wpn_supply_drop_ally".into()),
                Some("t6_wpn_supply_drop_axis".into()),
            ],
            ..Default::default()
        }
    }

    fn models(&self) -> impl Iterator<Item = &Option<String>> {
        [&self.neutral_flag, &self.bomb]
            .into_iter()
            .chain(&self.flag)
            .chain(&self.flag_carry)
            .chain(&self.crate_model)
            .chain(&self.crate_overlay)
    }

    pub fn model_names(&self) -> impl Iterator<Item = &str> {
        self.models().filter_map(|name| name.as_deref())
    }

    pub fn is_complete(&self) -> bool {
        self.models()
            .chain(&self.flag_base_fx)
            .chain([
                &self.bomb_explosion_fx,
                &self.plant_weapon,
                &self.defuse_weapon,
            ])
            .all(|name| name.as_deref().is_some_and(|s| !s.is_empty()))
    }

    pub fn model_pairs<'a>(&'a self, other: &'a Self) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.models()
            .zip(other.models())
            .filter_map(|(a, b)| Some((a.as_deref()?, b.as_deref()?)))
    }

    pub fn weapon_pairs<'a>(&'a self, other: &'a Self) -> impl Iterator<Item = (&'a str, &'a str)> {
        [&self.plant_weapon, &self.defuse_weapon]
            .into_iter()
            .zip([&other.plant_weapon, &other.defuse_weapon])
            .filter_map(|(a, b)| Some((a.as_deref()?, b.as_deref()?)))
    }

    pub fn fx_pairs<'a>(&'a self, other: &'a Self) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.flag_base_fx
            .iter()
            .zip(&other.flag_base_fx)
            .chain(std::iter::once((
                &self.bomb_explosion_fx,
                &other.bomb_explosion_fx,
            )))
            .filter_map(|(a, b)| Some((a.as_deref()?, b.as_deref()?)))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Resource)]
pub struct SessionTeamSettings(pub MapTeamSettings);

fn table_icon(
    table: &CapturedStringTable,
    namespace: asset_core::AssetNamespace,
    charset: Option<&str>,
) -> Option<asset_core::AssetKey> {
    let charset = charset.filter(|s| !s.is_empty())?;
    let cell = table.lookup_col(charset, FACTION_ICON_COL);
    asset_core::AssetKey::new(namespace, asset_core::AssetKind::Material, cell).ok()
}

#[must_use]
pub fn team_settings(
    table: &CapturedStringTable,
    namespace: asset_core::AssetNamespace,
    allieschar: Option<&str>,
    axischar: Option<&str>,
) -> MapTeamSettings {
    let color = |charset: &str| {
        let rgb = [14, 15, 16].map(|column| table.lookup_col(charset, column).parse::<f32>().ok());
        let rgb = [rgb[0]?, rgb[1]?, rgb[2]?];
        rgb.iter().all(|v| v.is_finite()).then_some(rgb)
    };
    let prefix = |charset: &str| {
        let prefix = table.lookup_col(charset, FACTION_VOICE_PREFIX_COL);
        (!prefix.is_empty()).then_some(prefix)
    };
    let voice = |charset: &str| prefix(charset).map(|prefix| format!("{prefix}1mc_"));
    let strings = |charset: &str| {
        let key = |column| {
            let key = table.lookup_col(charset, column);
            (!key.is_empty()).then(|| key.to_owned())
        };
        FactionStrings {
            name: key(1),
            eliminated: key(3),
            forfeited: key(4),
        }
    };
    let music = |charset: Option<&str>| {
        charset
            .and_then(prefix)
            .map_or_else(TeamMusic::default, TeamMusic::prefixed)
    };
    MapTeamSettings {
        allies_color: allieschar.and_then(color),
        axis_color: axischar.and_then(color),
        allies: table_icon(table, namespace, allieschar),
        axis: table_icon(table, namespace, axischar),
        allies_name: allieschar.and_then(|name| {
            asset_core::AssetKey::new(
                namespace,
                asset_core::AssetKind::Localize,
                table.lookup_col(name, 2),
            )
            .ok()
        }),
        axis_name: axischar.and_then(|name| {
            asset_core::AssetKey::new(
                namespace,
                asset_core::AssetKind::Localize,
                table.lookup_col(name, 2),
            )
            .ok()
        }),
        allies_charset: allieschar.map(str::to_owned),
        axis_charset: axischar.map(str::to_owned),
        allies_voice: allieschar.and_then(voice),
        axis_voice: axischar.and_then(voice),
        allies_music: music(allieschar),
        axis_music: music(axischar),
        allies_strings: allieschar.map_or_else(FactionStrings::default, strings),
        axis_strings: axischar.map_or_else(FactionStrings::default, strings),
        ..Default::default()
    }
}

#[must_use]
pub fn team_settings_for_zone(
    table: &CapturedStringTable,
    namespace: asset_core::AssetNamespace,
    arena_text: Option<&str>,
    zone: &str,
) -> MapTeamSettings {
    if zone.is_empty() {
        return MapTeamSettings::default();
    }
    let Some(row) = arena_text.and_then(|text| arena_charsets(text, zone)) else {
        return MapTeamSettings::default();
    };
    team_settings(
        table,
        namespace,
        row.allieschar.as_deref(),
        row.axischar.as_deref(),
    )
}

fn is_basemaps_arena_name(name: &str) -> bool {
    let n = name.replace('\\', "/");
    n.eq_ignore_ascii_case("mp/basemaps.arena")
        || n.rsplit('/')
            .next()
            .is_some_and(|leaf| leaf.eq_ignore_ascii_case("basemaps.arena"))
}

fn is_faction_table_name(name: &str) -> bool {
    name.replace('\\', "/")
        .eq_ignore_ascii_case("mp/factiontable.csv")
}

pub fn load_iw5_team_sources(zone_path: &Path) -> (Option<String>, Option<CapturedStringTable>) {
    let Ok(image) = crate::open_zone(zone_path) else {
        return (None, None);
    };
    let Ok(header) = image.iw5_header() else {
        return (None, None);
    };
    let mut memory = crate::Iw5ZoneMemory::for_header(&header);
    let Ok(mut stream) = memory.stream(&image.bytes) else {
        return (None, None);
    };
    let mut sink = Iw5TeamIconSink::default();
    let _ = fastfile_iw5::load_zone(&mut stream, &mut sink);
    (sink.arena_text, sink.faction_table)
}

#[derive(Default)]
struct Iw5TeamIconSink {
    arena_text: Option<String>,
    faction_table: Option<CapturedStringTable>,
}

impl fastfile_iw5::AssetSink for Iw5TeamIconSink {
    fn set_script_strings(&mut self, _strings: fastfile_iw5::ScriptStrings) {}
    fn load_asset(
        &mut self,
        s: &mut fastfile_iw5::ZoneStream<'_>,
        _index: usize,
        ty: fastfile_iw5::AssetType,
        slot: fastfile_iw5::Ptr,
    ) -> fastfile_iw5::Result<()> {
        fastfile_iw5::load_asset_at_observed(s, ty, slot, self)?;
        Ok(())
    }
}

impl fastfile_iw5::AssetLinkSink for Iw5TeamIconSink {
    fn loaded(
        &mut self,
        _s: &fastfile_iw5::ZoneStream<'_>,
        _ty: fastfile_iw5::AssetType,
        _slot: fastfile_iw5::Ptr,
        _insert: Option<fastfile_iw5::Ptr>,
    ) -> fastfile_iw5::Result<()> {
        Ok(())
    }
    fn alias(
        &mut self,
        _ty: fastfile_iw5::AssetType,
        _slot: fastfile_iw5::Ptr,
        _target: fastfile_iw5::Ptr,
    ) -> fastfile_iw5::Result<()> {
        Ok(())
    }
    fn capture_raw_file(
        &mut self,
        name: &str,
        data: &[u8],
        zlib: bool,
    ) -> fastfile_iw5::Result<()> {
        if is_basemaps_arena_name(name) {
            if let Some(text) = crate::decode_rawfile_text(data, zlib) {
                self.arena_text = Some(text);
            }
        }
        Ok(())
    }
    fn capture_string_table(
        &mut self,
        s: &fastfile_iw5::ZoneStream<'_>,
        header: fastfile_iw5::Ptr,
    ) -> fastfile_iw5::Result<()> {
        let name = match s.ptr_at(header, 0).ok() {
            Some(fastfile_iw5::ZonePtr::Offset(p)) => {
                s.cstr(s.resolve_alias(p)).ok().unwrap_or("").to_owned()
            }
            _ => return Ok(()),
        };
        if !is_faction_table_name(&name) {
            return Ok(());
        }
        let columns = s.i32_at(header, s.layout(4, 8)).unwrap_or(0).max(0) as usize;
        let rows = s.i32_at(header, s.layout(8, 12)).unwrap_or(0).max(0) as usize;
        let cells_n = columns.saturating_mul(rows);
        let arr = match s.ptr_at(header, s.layout(12, 16)).ok() {
            Some(fastfile_iw5::ZonePtr::Offset(p)) => s.resolve_alias(p),
            _ => {
                self.faction_table = Some(CapturedStringTable {
                    name,
                    columns,
                    rows,
                    cells: Vec::new(),
                });
                return Ok(());
            }
        };
        let mut cells = Vec::with_capacity(cells_n);
        let cell_sz = s.layout(fastfile_iw5::size::STRING_TABLE_CELL, 16);
        for i in 0..cells_n {
            let cell = arr.at(i * cell_sz);
            let value = match s.ptr_at(cell, 0).ok() {
                Some(fastfile_iw5::ZonePtr::Offset(p)) => {
                    s.cstr(s.resolve_alias(p)).ok().unwrap_or("").to_owned()
                }
                _ => String::new(),
            };
            cells.push(value);
        }
        self.faction_table = Some(CapturedStringTable {
            name,
            columns,
            rows,
            cells,
        });
        Ok(())
    }
}

#[must_use]
pub fn arena_charsets(text: &str, map: &str) -> Option<ArenaCharsets> {
    parse_arena(text)
        .into_iter()
        .find(|row| row.map.eq_ignore_ascii_case(map))
}

#[must_use]
pub fn arena_entry(text: &str, map: &str) -> Option<std::collections::BTreeMap<String, String>> {
    for block in text.split('{').skip(1) {
        let body = block.split('}').next().unwrap_or(block);
        let mut entry = std::collections::BTreeMap::new();
        for raw in body.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            let mut parts = line.split_whitespace();
            let Some(key) = parts.next() else {
                continue;
            };
            let value = unquote(parts.collect::<Vec<_>>().join(" ").as_str());
            entry.insert(key.to_ascii_lowercase(), value);
        }
        if entry
            .get("map")
            .is_some_and(|name| name.eq_ignore_ascii_case(map))
        {
            return Some(entry);
        }
    }
    None
}

#[must_use]
pub fn parse_arena(text: &str) -> Vec<ArenaCharsets> {
    let mut rows = Vec::new();
    for block in text.split('{').skip(1) {
        let body = block.split('}').next().unwrap_or(block);
        let mut map = None;
        let mut allieschar = None;
        let mut axischar = None;
        for raw in body.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            let mut parts = line.split_whitespace();
            let Some(key) = parts.next() else {
                continue;
            };
            let value = unquote(parts.collect::<Vec<_>>().join(" ").as_str());
            if value.is_empty() {
                continue;
            }
            match key {
                "map" => map = Some(value),
                "allieschar" => allieschar = Some(value),
                "axischar" => axischar = Some(value),
                _ => {}
            }
        }
        if let Some(map) = map {
            rows.push(ArenaCharsets {
                map,
                allieschar,
                axischar,
            });
        }
    }
    rows
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').to_owned()
}

#[must_use]
pub fn t5_teamset_key_from_rawfile(name: &str) -> Option<&str> {
    let file = name.rsplit(['/', '\\']).next()?;
    let stem = file.strip_prefix("_teamset_")?.strip_suffix(".gsc")?;
    (!stem.is_empty()).then_some(stem)
}

#[must_use]
pub fn t5_teamset_from_map_gsc(script: &str) -> Option<&str> {
    let lower = script.to_ascii_lowercase();
    let mut from = 0;
    while let Some(rel) = lower[from..].find("_teamset_") {
        let start = from + rel + "_teamset_".len();
        let Some(rest) = script.get(start..) else {
            return None;
        };
        let Some(end) = rest.find("::") else {
            from = start;
            continue;
        };
        let name = rest[..end].trim();
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            from = start + end + 2;
            continue;
        }
        if rest[end..].to_ascii_lowercase().starts_with("::level_init") {
            return Some(name);
        }
        from = start + end + 2;
    }
    None
}

fn script_dvar_string<'a>(script: &'a str, key: &str) -> Option<&'a str> {
    for line in script.lines() {
        let line = line.trim();
        let Some((function, args)) = line.split_once('(') else {
            continue;
        };
        if !function.trim().eq_ignore_ascii_case("setdvar") {
            continue;
        }
        let mut quotes = args.split('"');
        let (Some(before), Some(name), Some(between), Some(value)) =
            (quotes.next(), quotes.next(), quotes.next(), quotes.next())
        else {
            continue;
        };
        if !before.trim().is_empty() || !name.eq_ignore_ascii_case(key) {
            continue;
        }
        if !matches!(between.trim(), "," | ",&" | ", &") {
            continue;
        }
        return Some(value);
    }
    None
}

#[must_use]
pub fn t5_settings_from_teamset_gsc(script: &str) -> MapTeamSettings {
    let color = |key| {
        let mut values = script_dvar_string(script, key)?
            .split_whitespace()
            .map(str::parse::<f32>);
        let rgb = [
            values.next()?.ok()?,
            values.next()?.ok()?,
            values.next()?.ok()?,
        ];
        (values.next().is_none() && rgb.iter().all(|v| v.is_finite())).then_some(rgb)
    };
    MapTeamSettings {
        allies_color: color("g_TeamColor_Allies"),
        axis_color: color("g_TeamColor_Axis"),
        allies_name: script_dvar_string(script, "g_TeamName_Allies").and_then(|name| {
            asset_core::AssetKey::new(
                asset_core::AssetNamespace::T5,
                asset_core::AssetKind::Localize,
                name,
            )
            .ok()
        }),
        axis_name: script_dvar_string(script, "g_TeamName_Axis").and_then(|name| {
            asset_core::AssetKey::new(
                asset_core::AssetNamespace::T5,
                asset_core::AssetKind::Localize,
                name,
            )
            .ok()
        }),
        attackers: asset_audio::game_string_assignment(script, "attackers").map(str::to_owned),
        defenders: asset_audio::game_string_assignment(script, "defenders").map(str::to_owned),
        allies: crate::game_nested_string_assignment(script, &["icons", "allies"]).and_then(
            |name| {
                asset_core::AssetKey::new(
                    asset_core::AssetNamespace::T5,
                    asset_core::AssetKind::Material,
                    name,
                )
                .ok()
            },
        ),
        axis: crate::game_nested_string_assignment(script, &["icons", "axis"]).and_then(|name| {
            asset_core::AssetKey::new(
                asset_core::AssetNamespace::T5,
                asset_core::AssetKind::Material,
                name,
            )
            .ok()
        }),
        allies_charset: asset_audio::game_string_assignment(script, "allies").map(str::to_owned),
        axis_charset: asset_audio::game_string_assignment(script, "axis").map(str::to_owned),
        allies_strings: teamset_strings(script, "allies"),
        axis_strings: teamset_strings(script, "axis"),
        allies_voice: crate::game_nested_string_assignment(script, &["voice", "allies"])
            .map(str::to_owned),
        axis_voice: crate::game_nested_string_assignment(script, &["voice", "axis"])
            .map(str::to_owned),
        allies_music: TeamMusic::t5(script, "allies"),
        axis_music: TeamMusic::t5(script, "axis"),
    }
}

fn teamset_strings(script: &str, team: &str) -> FactionStrings {
    let key = |suffix| {
        crate::game_nested_string_assignment(script, &["strings", &format!("{team}_{suffix}")])
            .map(str::to_owned)
    };
    FactionStrings {
        name: key("name"),
        eliminated: key("eliminated"),
        forfeited: key("forfeited"),
    }
}

#[must_use]
pub fn t5_teamset_from_rawfile(name: &str, data: &[u8], zlib_compressed: bool) -> Option<String> {
    if !asset_audio::is_map_main_script(name) {
        return None;
    }
    let text = crate::decode_rawfile_text(data, zlib_compressed)?;
    t5_teamset_from_map_gsc(&text).map(str::to_owned)
}

#[must_use]
pub fn t5_settings_from_teamset_rawfile(
    name: &str,
    data: &[u8],
    zlib_compressed: bool,
) -> Option<(String, MapTeamSettings)> {
    let key = t5_teamset_key_from_rawfile(name)?.to_owned();
    let text = crate::decode_rawfile_text(data, zlib_compressed)?;
    let icons = t5_settings_from_teamset_gsc(&text);
    (icons.allies.is_some() || icons.axis.is_some()).then_some((key, icons))
}

pub fn read_basemaps_arena(games_root: &Path) -> Option<String> {
    read_iwd_named(games_root, "mp/basemaps.arena").and_then(|bytes| String::from_utf8(bytes).ok())
}

pub fn read_iwd_named(games_root: &Path, want: &str) -> Option<Vec<u8>> {
    asset_transport::read_iwd_named(games_root, want)
}
