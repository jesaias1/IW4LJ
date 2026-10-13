use super::args::{int, string};
use super::entities::table_key;
use crate::script::{RoundScript, StringTable, Value};
use bevy_ecs::prelude::World;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(crate) fn table<'a>(
    tables: &'a BTreeMap<String, StringTable>,
    name: &str,
) -> Option<&'a StringTable> {
    tables.get(&table_key(name))
}

pub(super) fn perk_slot_code(world: &World, name: &str) -> Option<(usize, u32)> {
    let tables = &world.resource::<RoundScript>().tables;
    let table = table(tables, "mp/perkTable.csv")?;
    let row = table_search(table, 1, name)?;
    let slot = hud_iw4::get_perk_slot_index(table.cell(row, 5)?)?;
    let code = table.cell(row, 0)?.parse().ok()?;
    Some((slot, code))
}

pub(crate) fn table_search(table: &StringTable, column: usize, value: &str) -> Option<usize> {
    (0..table.rows).find(|&row| {
        table
            .cell(row, column)
            .is_some_and(|cell| cell.eq_ignore_ascii_case(value))
    })
}

pub(crate) fn table_lookup(world: &World, args: &[Value]) -> Result<String, String> {
    if args.len() != 4 {
        return Err("wrong number of parameters".into());
    }
    let tables = world.resource::<RoundScript>().tables.clone();
    let name = string(args, 0)?;
    let column = int(args, 1)?;
    let value = string(args, 2)?;
    let result = int(args, 3)?;
    let Some(table) = table(&tables, &name) else {
        return Ok(String::new());
    };
    if column < 0 || result < 0 {
        return Ok(String::new());
    }
    Ok(table_search(table, column as usize, &value)
        .and_then(|row| table.cell(row, result as usize))
        .unwrap_or("")
        .to_owned())
}

pub(crate) fn table_lookup_by_row(world: &World, args: &[Value]) -> Result<Arc<str>, String> {
    let tables = world.resource::<RoundScript>().tables.clone();
    let name = string(args, 0)?;
    let (row, column) = (int(args, 1)?, int(args, 2)?);
    Ok(table(&tables, &name)
        .filter(|_| row >= 0 && column >= 0)
        .and_then(|t| t.cell(row as usize, column as usize))
        .unwrap_or("")
        .into())
}
