use bevy::prelude::*;

#[derive(Message, Clone, Debug)]
pub struct UiPlaySound {
    pub alias: String,
}

#[derive(Message, Clone, Debug)]
pub struct UiPlayMusic {
    pub alias: String,
}

#[derive(Message, Clone, Debug, Default)]
pub struct UiStopMusic;
