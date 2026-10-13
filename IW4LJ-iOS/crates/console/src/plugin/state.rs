use bevy::prelude::*;

use crate::ConsoleEditor;

#[derive(Resource, Clone)]
pub struct ConsoleSettings {
    pub log_capacity: usize,

    pub height: f32,

    pub width: f32,
}

impl Default for ConsoleSettings {
    fn default() -> Self {
        Self {
            log_capacity: 12,
            height: 360.0,
            width: 780.0,
        }
    }
}

#[derive(Resource)]
pub struct ConsoleFont(pub Handle<Font>);

#[derive(Resource, Default)]
pub struct ConsoleState {
    pub open: bool,
    pub editor: ConsoleEditor,
    pub log: Vec<String>,

    pub history: Vec<String>,

    pub history_cursor: Option<usize>,

    pub history_draft: String,

    pub prompt_sel_n: usize,

    pub scroll_sel_n: usize,

    pub scroll_anchor: Option<usize>,

    pub scroll_focus: usize,

    pub scroll_gesture: bool,

    pub clipboard_write: Option<bool>,

    pub copy_source: Option<&'static str>,

    pub last_hit_x: Option<f32>,
    pub last_hit_y: Option<f32>,
    pub last_hit_char: Option<usize>,

    pub pending_feed: Option<String>,

    pub pending_os_paste: bool,

    pub feed_n: usize,

    pub clipboard_read: Option<&'static str>,
}

impl ConsoleState {
    pub fn echo(&mut self, line: impl Into<String>, capacity: usize) {
        self.log.push(line.into());
        if self.log.len() > capacity {
            let drop = self.log.len() - capacity;
            self.log.drain(0..drop);
        }
    }

    pub(super) fn leave_history_browse(&mut self) {
        self.history_cursor = None;
        self.history_draft.clear();
    }

    pub(super) fn history_recall(&mut self, older: bool) -> bool {
        if self.history.is_empty() {
            return false;
        }
        match self.history_cursor {
            None => {
                if !older || !self.editor.line.is_empty() {
                    return false;
                }
                self.history_draft = self.editor.line.clone();
                self.history_cursor = Some(self.history.len() - 1);
            }
            Some(index) => {
                if older {
                    if index == 0 {
                        return true;
                    }
                    self.history_cursor = Some(index - 1);
                } else if index + 1 >= self.history.len() {
                    let draft = std::mem::take(&mut self.history_draft);
                    self.history_cursor = None;
                    super::set_editor_line(&mut self.editor, &draft);
                    return true;
                } else {
                    self.history_cursor = Some(index + 1);
                }
            }
        }
        if let Some(index) = self.history_cursor {
            let line = self.history[index].clone();
            super::set_editor_line(&mut self.editor, &line);
        }
        true
    }
}
