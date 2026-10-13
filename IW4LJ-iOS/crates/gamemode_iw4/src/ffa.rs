pub const GAMETYPE_TOKEN: &str = "dm";

pub const GAMETYPE_DIALOG_LINE: &str = "freeforall";

pub const DISPLAY_NAME: &str = "FREE FOR ALL";

pub const SCORE_KILL_POINTS: i32 = 50;

pub const SCORE_LIMIT: i32 = 1000;

pub const TIME_LIMIT_MS: u32 = 600_000;

pub const ROUND_LIMIT: i32 = 1;

pub const SPAWN_CLASSNAME: &str = "mp_dm_spawn";

pub const START_SPAWN_CLASSNAME: &str = "mp_dm_spawn_start";

pub const fn ffa_player_is_better(
    challenger_score: i32,
    challenger_deaths: i32,
    incumbent_score: i32,
    incumbent_deaths: i32,
) -> bool {
    if challenger_score > incumbent_score {
        return true;
    }
    if incumbent_score > challenger_score {
        return false;
    }
    challenger_deaths < incumbent_deaths
}

pub const fn ffa_highest_scoring_index(ranked_len: usize) -> Option<usize> {
    if ranked_len == 0 { None } else { Some(0) }
}
