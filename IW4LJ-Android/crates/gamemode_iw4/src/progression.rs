extern crate alloc;

use alloc::{collections::BTreeMap, string::String, vec::Vec};

pub trait ProgressionTable {
    fn rows(&self) -> usize;
    fn cell(&self, row: usize, column: usize) -> &str;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RankProgression {
    thresholds: Vec<(u32, i32)>,
}

impl RankProgression {
    pub fn capture(table: &impl ProgressionTable) -> Result<Self, &'static str> {
        let mut thresholds = Vec::new();
        for row in 0..table.rows() {
            let Ok(rank) = table.cell(row, 0).parse::<u32>() else {
                continue;
            };
            let xp = table
                .cell(row, 2)
                .parse::<i32>()
                .map_err(|_| "rank.invalid_threshold")?;
            if xp < 0 {
                return Err("rank.invalid_threshold");
            }
            thresholds.push((rank, xp));
        }
        thresholds.sort_unstable_by_key(|&(rank, _)| rank);
        if thresholds.first() != Some(&(0, 0))
            || thresholds
                .windows(2)
                .any(|pair| pair[1].0 != pair[0].0 + 1 || pair[1].1 <= pair[0].1)
        {
            return Err("rank.invalid_table");
        }
        Ok(Self { thresholds })
    }

    pub fn thresholds(&self) -> &[(u32, i32)] {
        &self.thresholds
    }

    pub fn rank(&self, experience: i32) -> Option<u32> {
        self.thresholds
            .iter()
            .rev()
            .find(|&&(_, xp)| xp <= experience.max(0))
            .map(|&(rank, _)| rank)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChallengeRequirement {
    pub name: String,
    pub tier: i32,
}

impl ChallengeRequirement {
    pub fn parse(name: &str) -> Self {
        let (name, tier) = name
            .rsplit_once('_')
            .filter(|(_, suffix)| suffix.len() == 1 && suffix.as_bytes()[0].is_ascii_digit())
            .map_or((name, 1), |(base, suffix)| {
                (base, i32::from(suffix.as_bytes()[0] - b'0'))
            });
        Self {
            name: name.into(),
            tier,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnlockRequirement {
    pub rank: u32,
    pub challenges: Vec<ChallengeRequirement>,
}

impl UnlockRequirement {
    pub fn capture(rank: &str, challenges: &str) -> Result<Self, &'static str> {
        Ok(Self {
            rank: rank.parse().map_err(|_| "unlock.invalid_rank")?,
            challenges: challenges
                .split(|c: char| c == ';' || c.is_ascii_whitespace())
                .filter(|name| !name.is_empty())
                .map(ChallengeRequirement::parse)
                .collect(),
        })
    }

    pub fn unlocked(
        &self,
        rank: u32,
        mut challenge_state: impl FnMut(&str) -> Option<i32>,
    ) -> bool {
        rank >= self.rank
            && self.challenges.iter().all(|challenge| {
                challenge_state(&challenge.name).is_some_and(|state| state > challenge.tier)
            })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayerProgression {
    pub experience: i32,
    pub prestige: i32,
    pub rank: u32,
    pub challenges: BTreeMap<String, i32>,
}

impl PlayerProgression {
    pub fn unlocked(&self, requirement: &UnlockRequirement) -> bool {
        requirement.unlocked(self.rank, |name| self.challenges.get(name).copied())
    }
}

pub fn custom_class_capacity(rank: u32, prestige: i32) -> usize {
    if rank < 3 {
        return 0;
    }
    custom_class_slot_count(prestige)
}

pub fn custom_class_slot_count(prestige: i32) -> usize {
    5 + ((prestige.clamp(0, 10) + 1) / 2) as usize
}
