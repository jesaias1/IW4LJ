use crate::{AccountSnapshot, PersistentDataError};
use bevy_ecs::prelude::Resource;
use gamemode_iw4::progression::{PlayerProgression, RankProgression, UnlockRequirement};
use std::sync::Arc;
use structured_data_iw4::{DataType, Definition, Key, Value};

pub(crate) fn read_progression(
    definition: &Definition,
    bytes: &[u8],
    ranks: &RankProgression,
) -> Result<PlayerProgression, PersistentDataError> {
    let read = |keys: &[Key<'_>]| -> Result<i32, PersistentDataError> {
        match definition.read(bytes, definition.lookup(keys)?)? {
            Value::Int(n) => Ok(n),
            _ => Err(PersistentDataError::Field(
                structured_data_iw4::Error::WrongValue,
            )),
        }
    };
    let experience = read(&[Key::Name("experience")])?;
    let prestige = read(&[Key::Name("prestige")])?;
    let rank = ranks
        .rank(experience)
        .ok_or(PersistentDataError::InvalidSchema)?;
    let mut challenges = std::collections::BTreeMap::new();
    let DataType::EnumArray(index) = definition.path_type(&[Key::Name("challengeState")])? else {
        return Err(PersistentDataError::InvalidSchema);
    };
    let enumeration = &definition.enums[definition.enum_arrays[index].enumeration];
    for entry in &enumeration.entries {
        let state = read(&[Key::Name("challengeState"), Key::Name(&entry.name)])?;
        if state != 0 {
            challenges.insert(entry.name.clone(), state);
        }
    }
    Ok(PlayerProgression {
        experience,
        prestige,
        rank,
        challenges,
    })
}

#[derive(Resource, Clone, Debug, Default)]
pub struct LocalPlayerData {
    definition: Option<Arc<Definition>>,
    ranks: RankProgression,
    snapshot: Option<AccountSnapshot>,
    initial_bytes: Vec<u8>,
    defaults: Option<crate::PlayerDataDefaults>,
    challenges: Option<crate::script::StringTable>,
    rank_tables: std::collections::BTreeMap<String, crate::script::StringTable>,
    pub progression: Option<PlayerProgression>,
}

impl LocalPlayerData {
    pub fn custom_class_available(&self, index: usize) -> bool {
        let Some(progression) = &self.progression else {
            return false;
        };
        index
            < gamemode_iw4::progression::custom_class_capacity(
                progression.rank,
                progression.prestige,
            )
    }

    pub fn unlock_data_ready(&self) -> bool {
        self.definition.is_some() && self.defaults.is_some() && self.challenges.is_some()
    }

    pub fn install(
        &mut self,
        definitions: &structured_data_iw4::DefinitionSet,
        ranks: RankProgression,
        snapshot: Option<&AccountSnapshot>,
    ) -> Result<(), PersistentDataError> {
        self.progression = None;
        self.ranks = ranks;
        self.definition = snapshot
            .and_then(|snapshot| {
                definitions.definitions.iter().find(|definition| {
                    definition.version == snapshot.version
                        && definition.checksum == snapshot.checksum
                })
            })
            .cloned()
            .map(Arc::new)
            .or_else(|| definitions.definitions.first().cloned().map(Arc::new));
        self.initial_bytes = self
            .definition
            .as_ref()
            .map_or_else(Vec::new, |definition| vec![0; definition.size]);
        self.snapshot = None;
        self.refresh(snapshot)
    }

    /// Whether [`refresh`](Self::refresh) with this snapshot would do nothing.
    /// Asked through `&self`, so a Bevy system can check before taking the
    /// resource mutably: a `ResMut` that is only dereferenced mutably counts as
    /// a change, and every system gated on that change then runs every frame.
    pub fn is_current(&self, snapshot: Option<&AccountSnapshot>) -> bool {
        self.snapshot.as_ref() == snapshot && self.progression.is_some()
    }

    pub fn refresh(
        &mut self,
        snapshot: Option<&AccountSnapshot>,
    ) -> Result<(), PersistentDataError> {
        if self.snapshot.as_ref() == snapshot && self.progression.is_some() {
            return Ok(());
        }
        let progression = match snapshot {
            Some(snapshot) => {
                let definition = self
                    .definition
                    .as_ref()
                    .ok_or(PersistentDataError::MissingSchema)?;
                if definition.version != snapshot.version
                    || definition.checksum != snapshot.checksum
                {
                    return Err(PersistentDataError::SchemaMismatch);
                }
                Some(read_progression(definition, &snapshot.bytes, &self.ranks)?)
            }
            None => Some(PlayerProgression::default()),
        };
        self.progression = progression;
        self.snapshot = snapshot.cloned();
        Ok(())
    }

    pub fn install_unlock_data(
        &mut self,
        defaults: Option<crate::PlayerDataDefaults>,
        challenges: Option<crate::script::StringTable>,
    ) {
        self.defaults = defaults;
        self.challenges = challenges;
    }

    /// The account's saved data with its prestige set (0 to 10), nothing else
    /// changed: the rank and unlocks a player earned stay theirs.
    pub fn prestige_snapshot(
        &self,
        snapshot: &AccountSnapshot,
        prestige: i32,
    ) -> Result<AccountSnapshot, PersistentDataError> {
        let definition = self
            .definition
            .as_ref()
            .ok_or(PersistentDataError::MissingSchema)?;
        if snapshot.version != definition.version || snapshot.checksum != definition.checksum {
            return Err(PersistentDataError::SchemaMismatch);
        }
        let mut updated = snapshot.clone();
        let mut dirty = vec![0; updated.bytes.len().div_ceil(8)];
        let changed = definition.write(
            &mut updated.bytes,
            &mut dirty,
            definition.lookup(&[Key::Name("prestige")])?,
            Value::Int(prestige.clamp(0, 10)),
        )?;
        if changed {
            updated.revision = updated
                .revision
                .checked_add(1)
                .ok_or(PersistentDataError::RevisionOverflow)?;
        }
        read_progression(definition, &updated.bytes, &self.ranks)?;
        Ok(updated)
    }

    pub fn unlock_snapshot(
        &self,
        account: crate::AccountId,
        snapshot: Option<&AccountSnapshot>,
        unlock: bool,
    ) -> Result<AccountSnapshot, PersistentDataError> {
        let definition = self
            .definition
            .as_ref()
            .ok_or(PersistentDataError::MissingSchema)?;
        let defaults = self
            .defaults
            .as_ref()
            .ok_or(PersistentDataError::MissingDefaults)?;
        let initial = crate::persistent_defaults::initialize(definition, defaults)?;
        let mut updated = match snapshot {
            Some(snapshot)
                if snapshot.account == account
                    && snapshot.version == definition.version
                    && snapshot.checksum == definition.checksum =>
            {
                snapshot.clone()
            }
            Some(_) => return Err(PersistentDataError::SchemaMismatch),
            None => AccountSnapshot {
                account,
                revision: 0,
                version: definition.version,
                checksum: definition.checksum,
                bytes: initial.clone(),
                skills: Default::default(),
            },
        };
        let xp = if unlock {
            self.ranks
                .thresholds()
                .last()
                .ok_or(PersistentDataError::InvalidSchema)?
                .1
        } else {
            0
        };
        let mut dirty = vec![0; updated.bytes.len().div_ceil(8)];
        let mut changed = definition.write(
            &mut updated.bytes,
            &mut dirty,
            definition.lookup(&[Key::Name("experience")])?,
            Value::Int(xp),
        )?;
        if !unlock {
            changed |= definition.write(
                &mut updated.bytes,
                &mut dirty,
                definition.lookup(&[Key::Name("prestige")])?,
                Value::Int(0),
            )?;
        }
        let table = if unlock {
            Some(
                self.challenges
                    .as_ref()
                    .ok_or(PersistentDataError::MissingDefaults)?,
            )
        } else {
            None
        };
        for field in ["challengeState", "challengeProgress"] {
            let DataType::EnumArray(index) = definition.path_type(&[Key::Name(field)])? else {
                return Err(PersistentDataError::InvalidSchema);
            };
            let enumeration = &definition.enums[definition.enum_arrays[index].enumeration];
            for entry in &enumeration.entries {
                let keys = [Key::Name(field), Key::Name(&entry.name)];
                let lookup = definition.lookup(&keys)?;
                let value = if let Some(table) = table {
                    let Some(row) =
                        crate::script::host::tables::table_search(table, 0, &entry.name)
                    else {
                        continue;
                    };
                    let mut target = 0;
                    let mut state = 1;
                    for tier in 1..=10 {
                        let next = table
                            .cell(row, 6 + (tier - 1) * 2)
                            .and_then(|value| value.parse::<i32>().ok())
                            .unwrap_or(0);
                        if next <= 0 {
                            break;
                        }
                        target = next;
                        state = tier as i32 + 1;
                    }
                    if target == 0 {
                        return Err(PersistentDataError::InvalidDefaults);
                    }
                    Value::Int(if field == "challengeState" {
                        state
                    } else {
                        target
                    })
                } else {
                    definition.read(&initial, lookup)?
                };
                changed |= definition.write(&mut updated.bytes, &mut dirty, lookup, value)?;
            }
        }
        if changed {
            updated.revision = updated
                .revision
                .checked_add(1)
                .ok_or(PersistentDataError::RevisionOverflow)?;
        }
        read_progression(definition, &updated.bytes, &self.ranks)?;
        Ok(updated)
    }

    pub fn install_rank_table(&mut self, name: &str, table: crate::script::StringTable) {
        self.rank_tables.insert(name.to_ascii_lowercase(), table);
    }

    pub fn table_cell(&self, name: &str, row: i32, column: i32) -> Option<&str> {
        let table = self.rank_tables.get(&name.to_ascii_lowercase())?;
        Some(
            usize::try_from(row)
                .ok()
                .zip(usize::try_from(column).ok())
                .and_then(|(row, column)| table.cell(row, column))
                .unwrap_or(""),
        )
    }

    pub fn table_lookup(
        &self,
        name: &str,
        column: i32,
        key: &str,
        result_column: i32,
    ) -> Option<&str> {
        let table = self.rank_tables.get(&name.to_ascii_lowercase())?;
        let row = usize::try_from(column)
            .ok()
            .and_then(|column| crate::script::host::tables::table_search(table, column, key));
        Some(
            row.zip(usize::try_from(result_column).ok())
                .and_then(|(row, column)| table.cell(row, column))
                .unwrap_or(""),
        )
    }

    pub fn read(&self, keys: &[Key<'_>]) -> Result<Value<'_>, PersistentDataError> {
        let bytes = self
            .snapshot
            .as_ref()
            .map_or(self.initial_bytes.as_slice(), |snapshot| {
                snapshot.bytes.as_slice()
            });
        let definition = self
            .definition
            .as_ref()
            .ok_or(PersistentDataError::MissingSchema)?;
        Ok(definition.read(bytes, definition.lookup(keys)?)?)
    }
}

pub(crate) fn unlocked(
    world: &bevy_ecs::prelude::World,
    client: crate::ClientId,
    requirement: &UnlockRequirement,
) -> Result<bool, PersistentDataError> {
    let store = world.resource::<crate::PersistentDataStore>();
    let xp = match store.read(client, &[Key::Name("experience")])? {
        Value::Int(xp) => xp,
        _ => return Err(PersistentDataError::InvalidBuffer),
    };
    let state = world
        .get::<crate::world::SimState>(crate::frame::state_entity(world))
        .ok_or(PersistentDataError::MissingSchema)?;
    let rank = state
        .rank_for_xp(xp)
        .ok_or(PersistentDataError::InvalidSchema)?;
    Ok(requirement.unlocked(rank, |name| {
        match store.read(client, &[Key::Name("challengeState"), Key::Name(name)]) {
            Ok(Value::Int(state)) => Some(state),
            _ => None,
        }
    }))
}

pub(crate) fn item_requirement(
    world: &bevy_ecs::prelude::World,
    name: &str,
) -> Result<UnlockRequirement, String> {
    let tables = &world.resource::<crate::script::RoundScript>().tables;
    let table = crate::script::host::tables::table(tables, "mp/unlockTable.csv")
        .ok_or("unlock.missing_table")?;
    let Some(row) = crate::script::host::tables::table_search(table, 0, name) else {
        return Ok(Default::default());
    };
    UnlockRequirement::capture(
        table.cell(row, 2).unwrap_or(""),
        table.cell(row, 3).unwrap_or(""),
    )
    .map_err(str::to_owned)
}

pub(crate) fn validate_class(
    world: &crate::frame::FrameWorld,
    client: crate::ClientId,
    def: &crate::ClassDef,
) -> Result<(), crate::ClassRejectReason> {
    let reject = crate::ClassRejectReason::LockedContent;
    let store = world.ecs_ref().resource::<crate::PersistentDataStore>();
    let prestige = match store.read(client, &[Key::Name("prestige")]) {
        Ok(Value::Int(n)) => n,
        _ => return Err(reject),
    };
    let feature = item_requirement(world.ecs_ref(), "cac").map_err(|_| reject)?;
    if !unlocked(world.ecs_ref(), client, &feature).map_err(|_| reject)? {
        return Err(reject);
    }
    let index = def.id.0 as usize;
    if index >= gamemode_iw4::progression::custom_class_slot_count(prestige) {
        return Err(reject);
    }
    for weapon in def
        .weapon_slot_ids()
        .into_iter()
        .filter(|&weapon| weapon != 0)
    {
        let requirement = world
            .weapon_unlock_requirement(weapon)
            .ok_or(reject)?
            .as_ref()
            .map_err(|_| reject)?;
        if !unlocked(world.ecs_ref(), client, requirement).map_err(|_| reject)? {
            return Err(reject);
        }
    }
    for name in def
        .perks
        .iter()
        .filter_map(|&id| crate::match_state::class_catalog_perk_name(id))
        .chain(
            (!def.deathstreak.is_empty() && def.deathstreak != "specialty_null")
                .then_some(def.deathstreak.as_str()),
        )
    {
        let requirement = item_requirement(world.ecs_ref(), name).map_err(|_| reject)?;
        if !unlocked(world.ecs_ref(), client, &requirement).map_err(|_| reject)? {
            return Err(reject);
        }
    }
    if def.perks[0] == 12
        && world
            .weapon_setup(def.secondary)
            .is_some_and(|setup| setup.attachments.len() > 1)
    {
        let requirement =
            item_requirement(world.ecs_ref(), "specialty_secondarybling").map_err(|_| reject)?;
        if !unlocked(world.ecs_ref(), client, &requirement).map_err(|_| reject)? {
            return Err(reject);
        }
    }
    Ok(())
}
