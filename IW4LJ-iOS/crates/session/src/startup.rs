use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct StartupCommands {
    pub lines: Vec<String>,
}

#[derive(Resource, Default)]
pub struct PendingConsoleLines(pub Vec<String>);
