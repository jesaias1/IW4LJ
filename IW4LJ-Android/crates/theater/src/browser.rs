use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures_lite::future};
use frame::{AppScreen, LaunchIdentity};
use replay::Playback;

use crate::editor::{EditorAction, TheaterState};
use crate::project::inspect;

#[derive(Clone)]
struct FilmEntry {
    path: PathBuf,
    name: String,
    kind: &'static str,
    zone: String,
    frames: u64,
    error: Option<String>,
    preview: Option<PathBuf>,
    modified: SystemTime,
}

#[derive(Resource, Default)]
pub(crate) struct Browser {
    pub open: bool,
    pub(crate) actions: Vec<BrowserButton>,
    task: Option<Task<Result<Vec<FilmEntry>, String>>>,
    entries: Vec<FilmEntry>,
    page: usize,
    redraw: bool,
    error: Option<String>,
}

#[derive(Component)]
pub(crate) struct BrowserRoot;

#[derive(Component, Clone)]
pub(crate) enum BrowserButton {
    Back,
    Refresh,
    Previous,
    Next,
    Play(PathBuf),
    Unavailable,
}

pub(crate) fn register(app: &mut App) {
    app.init_resource::<Browser>()
        .add_observer(activate_button)
        .add_systems(
            Update,
            (browser_actions, scan_library, draw_browser)
                .chain()
                .in_set(frame::ClientSet::Ui)
                .after(crate::navigation::TheaterNavigation),
        );
}

fn preview_bytes(path: &Path) -> Option<Vec<u8>> {
    let preview = path.with_extension("png");
    let meta = preview.metadata().ok()?;
    if meta.len() > 8 * 1024 * 1024 {
        return None;
    }
    let bytes = std::fs::read(preview).ok()?;
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    (width > 0 && height > 0 && width <= 4096 && height <= 4096).then_some(bytes)
}

fn scan(root: PathBuf) -> Result<Vec<FilmEntry>, String> {
    let mut candidates = Vec::new();
    let demos = root.join("demos");
    if demos.is_dir() {
        for entry in std::fs::read_dir(demos).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().is_some_and(|ext| ext == "iw4ldemo") {
                candidates.push((path, "Film"));
            }
        }
    }
    let clips = root.join("clips");
    if clips.is_dir() {
        for entry in std::fs::read_dir(clips).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_name() == "LATEST"
                || entry.file_type().map_err(|e| e.to_string())?.is_symlink()
            {
                continue;
            }
            let path = entry.path().join(replay::CLIP_DEMO_FILE);
            if path.is_file() {
                candidates.push((path, "Clip"));
            }
        }
    }
    let mut entries = Vec::new();
    for (path, kind) in candidates {
        let name = if kind == "Clip" {
            path.parent().and_then(Path::file_name)
        } else {
            path.file_stem()
        }
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Unnamed recording".into());
        let result = Playback::open_path(&path)
            .map_err(|e| e.to_string())
            .and_then(|p| {
                let zone = p
                    .identity()
                    .zone_name()
                    .map(str::to_owned)
                    .ok_or("recording has no map identity; re-record it to use Theater")?;
                let info = inspect(&path)?;
                if info.frames == 0 {
                    return Err("recording is empty".into());
                }
                Ok((zone, info.frames))
            });
        let (zone, frames, error) = match result {
            Ok((zone, frames)) => (zone, frames, None),
            Err(error) => (String::new(), 0, Some(error)),
        };
        entries.push(FilmEntry {
            preview: path
                .with_extension("png")
                .is_file()
                .then(|| path.with_extension("png")),
            modified: path
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH),
            path,
            name,
            kind,
            zone,
            frames,
            error,
        });
    }
    entries.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.path.cmp(&b.path))
    });
    Ok(entries)
}

fn scan_library(identity: Option<Res<LaunchIdentity>>, mut browser: ResMut<Browser>) {
    if !browser.open {
        return;
    }
    if let Some(task) = browser.task.as_mut() {
        if let Some(result) = future::block_on(future::poll_once(task)) {
            browser.task = None;
            match result {
                Ok(entries) => {
                    browser.entries = entries;
                    browser.error = None;
                }
                Err(error) => {
                    browser.entries.clear();
                    browser.error = Some(error);
                }
            }
            browser.redraw = true;
        }
    } else if browser.redraw && browser.entries.is_empty() && browser.error.is_none() {
        let Some(identity) = identity else {
            browser.error = Some("recording directory unavailable".into());
            return;
        };
        let root = identity.artifacts.clone();
        browser.task = Some(AsyncComputeTaskPool::get().spawn(async move { scan(root) }));
    }
}

fn activate_button(
    activation: On<bevy::ui_widgets::Activate>,
    buttons: Query<&BrowserButton>,
    mut browser: ResMut<Browser>,
) {
    if let Ok(button) = buttons.get(activation.entity) {
        browser.actions.push(button.clone());
    }
}

fn browser_actions(
    mut browser: ResMut<Browser>,
    mut editor: ResMut<TheaterState>,
    keys: Res<ButtonInput<KeyCode>>,
    mut menus: MessageWriter<frame::UiMenuRequest>,
) {
    if !browser.open {
        return;
    }
    let mut actions = std::mem::take(&mut browser.actions);
    if keys.just_pressed(KeyCode::Escape) {
        actions.push(BrowserButton::Back);
    }
    for action in actions {
        match action {
            BrowserButton::Back => {
                browser.open = false;
                menus.write(frame::UiMenuRequest::Close(
                    "iw4l_theater_background".into(),
                ));
                menus.write(frame::UiMenuRequest::Open("iw4l_main".into()));
            }
            BrowserButton::Refresh => {
                browser.entries.clear();
                browser.error = None;
                browser.task = None;
                browser.page = 0;
                browser.redraw = true;
            }
            BrowserButton::Previous => {
                browser.page = browser.page.saturating_sub(1);
                browser.redraw = true;
            }
            BrowserButton::Next => {
                if (browser.page + 1) * 6 < browser.entries.len() {
                    browser.page += 1;
                    browser.redraw = true;
                }
            }
            BrowserButton::Play(path) => {
                editor.actions.push(EditorAction::Open(path));
            }
            BrowserButton::Unavailable => {}
        }
    }
}

fn text(
    parent: &mut ChildSpawnerCommands,
    value: impl Into<String>,
    font: &Handle<Font>,
    size: f32,
) {
    parent.spawn((
        Text::new(value),
        TextLayout::default().with_linebreak(bevy::text::LineBreak::WordOrCharacter),
        ui::game_text_font(font, size),
        TextColor(Color::WHITE),
        bevy::picking::Pickable::IGNORE,
    ));
}

fn button(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    action: BrowserButton,
    font: &Handle<Font>,
) {
    parent
        .spawn((
            bevy::ui_widgets::Button,
            bevy::picking::hover::Hovered::default(),
            action,
            Node {
                padding: UiRect::all(Val::Px(7.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.18, 0.20)),
        ))
        .with_children(|p| text(p, title, font, 16.0));
}

fn draw_browser(
    mut commands: Commands,
    mut browser: ResMut<Browser>,
    font: Res<ui::GameUiFont>,
    mut images: ResMut<Assets<Image>>,
    roots: Query<Entity, With<BrowserRoot>>,
    screen: Res<AppScreen>,
    mut menus: MessageWriter<frame::UiMenuRequest>,
) {
    if !browser.open || *screen != AppScreen::MainMenu {
        for root in &roots {
            commands.entity(root).despawn();
        }
        return;
    }
    if !browser.redraw && !roots.is_empty() {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    browser.redraw = false;
    menus.write(frame::UiMenuRequest::Close("iw4l_main".into()));
    menus.write(frame::UiMenuRequest::Open("iw4l_theater_background".into()));
    let previews: Vec<_> = browser
        .entries
        .iter()
        .skip(browser.page * 6)
        .take(6)
        .map(|entry| {
            entry
                .preview
                .as_ref()
                .and_then(|path| preview_bytes(&path.with_extension("iw4ldemo")))
                .and_then(|bytes| {
                    Image::from_buffer(
                        &bytes,
                        bevy::image::ImageType::Extension("png"),
                        bevy::image::CompressedImageFormats::NONE,
                        true,
                        bevy::image::ImageSampler::linear(),
                        bevy::asset::RenderAssetUsages::default(),
                    )
                    .ok()
                })
                .map(|image| images.add(image))
        })
        .collect();
    commands.spawn((BrowserRoot, ui::UiLayer::Overlay, ui::UiLayerVisibility,
        GlobalZIndex(100), Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), padding: UiRect::all(Val::Px(24.0)), flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), ..default() }, BackgroundColor(Color::srgba(0.0,0.0,0.0,0.35))))
        .with_children(|root| {
            text(root, "THEATER  /  SAVED FILMS & CLIPS", &font.0, 24.0);
            root.spawn(Node { column_gap: Val::Px(10.0), ..default() }).with_children(|p| {
                button(p,"Back",BrowserButton::Back,&font.0);
                button(p,"Refresh",BrowserButton::Refresh,&font.0);
                button(p,"Previous",BrowserButton::Previous,&font.0);
                button(p,"Next",BrowserButton::Next,&font.0);
                text(p,format!("Page {} · {} recordings",browser.page+1,browser.entries.len()),&font.0,18.0);
            });
            if browser.task.is_some() { text(root,"Reading saved recordings…",&font.0,20.0); }
            else if let Some(error) = &browser.error { text(root,error,&font.0,20.0); }
            else if browser.entries.is_empty() { text(root,"No saved recordings. In a match, use record / stoprecord or clip to save a film.",&font.0,20.0); }
            root.spawn(Node { flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(12.0), row_gap: Val::Px(12.0), ..default() }).with_children(|grid| {
                for (entry, preview) in browser.entries.iter().skip(browser.page*6).take(6).zip(previews) {
                    grid.spawn((bevy::ui_widgets::Button, bevy::picking::hover::Hovered::default(),
                        if entry.error.is_some() { BrowserButton::Unavailable } else { BrowserButton::Play(entry.path.clone()) },
                        Node { width: Val::Percent(30.0), padding: UiRect::all(Val::Px(12.0)), flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), min_width: Val::Px(0.0), overflow: Overflow::clip_x(), ..default() }, BackgroundColor(Color::srgba(0.02,0.02,0.02,0.72)))).with_children(|card| {
                        if let Some(preview) = preview { card.spawn((ImageNode::new(preview), Node { width: Val::Percent(100.0), height: Val::Px(80.0), ..default() })); }
                        else { card.spawn((Node { width: Val::Percent(100.0), height: Val::Px(80.0), align_items: AlignItems::Center, justify_content: JustifyContent::Center, ..default() }, BackgroundColor(Color::srgb(0.12,0.14,0.15)))).with_children(|p| text(p,"No saved preview",&font.0,16.0)); }
                        text(card,&entry.name,&font.0,15.0);
                        if let Some(error) = &entry.error { text(card,format!("Unavailable: {error}"),&font.0,11.0); }
                        else {
                            text(card,format!("{} · {} · {:.1}s",entry.kind,entry.zone,entry.frames as f64 * net::AUTHORITY_MS as f64 / 1000.0),&font.0,15.0);
                            text(card,"A / Enter · PLAY",&font.0,15.0);
                        }
                    });
                }
            });
        });
}

impl Browser {
    pub fn show(&mut self) {
        self.open = true;
        self.entries.clear();
        self.task = None;
        self.error = None;
        self.page = 0;
        self.redraw = true;
    }
}
