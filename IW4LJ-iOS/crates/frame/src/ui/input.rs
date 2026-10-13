use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct UiBindingCapture {
    pub command: Option<String>,
    pub consumed_input: bool,
}

#[derive(Message, Clone, Debug)]
pub struct UiBindRequest {
    pub command: String,
}
