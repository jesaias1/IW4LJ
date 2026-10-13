//! Weapon skins: the images and short videos the skin tabs of the mod menu offer.
//!
//! A still skin is one 256x256 image. A video skin is sixteen 128x128 frames in
//! a 4x4 sheet, played at eight frames a second, so it costs a small upload a
//! few times a second rather than a decoder.

pub(crate) struct Skin {
    pub name: &'static str,
    pub video: bool,
    pub png: &'static [u8],
}

pub(crate) const SKINS: [Skin; 27] = [
    Skin {
        name: "ARCTIC SHARD",
        video: false,
        png: include_bytes!("../skins/t_arctic_shard.png"),
    },
    Skin {
        name: "BLACKGLASS",
        video: false,
        png: include_bytes!("../skins/t_blackglass.png"),
    },
    Skin {
        name: "BLACKSTAR",
        video: false,
        png: include_bytes!("../skins/t_blackstar.png"),
    },
    Skin {
        name: "CHROMAFLUX",
        video: false,
        png: include_bytes!("../skins/t_chromaflux.png"),
    },
    Skin {
        name: "CRYOSTORM",
        video: false,
        png: include_bytes!("../skins/t_cryostorm.png"),
    },
    Skin {
        name: "FROSTWYRM",
        video: false,
        png: include_bytes!("../skins/t_frostwyrm.png"),
    },
    Skin {
        name: "GHOST PEARL",
        video: false,
        png: include_bytes!("../skins/t_ghost_pearl.png"),
    },
    Skin {
        name: "GILDED ABYSS",
        video: false,
        png: include_bytes!("../skins/t_gilded_abyss.png"),
    },
    Skin {
        name: "GLACIER STATIC",
        video: false,
        png: include_bytes!("../skins/t_glacier_static.png"),
    },
    Skin {
        name: "GOLDEN DAMASCUS",
        video: false,
        png: include_bytes!("../skins/t_golden_damascus.png"),
    },
    Skin {
        name: "HELLFORGED",
        video: false,
        png: include_bytes!("../skins/t_hellforged.png"),
    },
    Skin {
        name: "JUNGLE CIRCUIT",
        video: false,
        png: include_bytes!("../skins/t_jungle_circuit.png"),
    },
    Skin {
        name: "NEON ANARCHY",
        video: false,
        png: include_bytes!("../skins/t_neon_anarchy.png"),
    },
    Skin {
        name: "NEON HAZARD",
        video: false,
        png: include_bytes!("../skins/t_neon_hazard.png"),
    },
    Skin {
        name: "NULLFORGE",
        video: false,
        png: include_bytes!("../skins/t_nullforge.png"),
    },
    Skin {
        name: "OIL SLICK",
        video: false,
        png: include_bytes!("../skins/t_oil_slick.png"),
    },
    Skin {
        name: "PRISM STATIC",
        video: false,
        png: include_bytes!("../skins/t_prism_static.png"),
    },
    Skin {
        name: "PRISMBREAK",
        video: false,
        png: include_bytes!("../skins/t_prismbreak.png"),
    },
    Skin {
        name: "REDSHIFT",
        video: false,
        png: include_bytes!("../skins/t_redshift.png"),
    },
    Skin {
        name: "RUST SERPENT",
        video: false,
        png: include_bytes!("../skins/t_rust_serpent.png"),
    },
    Skin {
        name: "VOID ATLAS",
        video: false,
        png: include_bytes!("../skins/t_void_atlas.png"),
    },
    Skin {
        name: "VOLCANIC ASH",
        video: false,
        png: include_bytes!("../skins/t_volcanic_ash.png"),
    },
    Skin {
        name: "WHITE OPAL",
        video: false,
        png: include_bytes!("../skins/t_white_opal.png"),
    },
    Skin {
        name: "XENOBLOOM",
        video: false,
        png: include_bytes!("../skins/t_xenobloom.png"),
    },
    Skin {
        name: "BLUE SMOKE (V)",
        video: true,
        png: include_bytes!("../skins/v_blue-smoke.png"),
    },
    Skin {
        name: "UNDERWORLD (V)",
        video: true,
        png: include_bytes!("../skins/v_underworld.png"),
    },
    Skin {
        name: "WHITE SMOKE (V)",
        video: true,
        png: include_bytes!("../skins/v_white-smoke.png"),
    },
];

/// The skin chosen in the menu, as an index into [`SKINS`]; `usize::MAX` for none.
static SELECTED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(usize::MAX);

fn select_unsaved(index: i32) {
    let value = usize::try_from(index).ok().filter(|i| *i < SKINS.len()).unwrap_or(usize::MAX);
    SELECTED.store(value, std::sync::atomic::Ordering::Relaxed);
}

/// Choose a skin (`-1` for none) and remember it for next time.
pub(crate) fn select(index: i32) {
    select_unsaved(index);
    crate::menu::write_setting(crate::menu::Setting::Skin, index);
}

pub(crate) fn selected() -> Option<usize> {
    let value = SELECTED.load(std::sync::atomic::Ordering::Relaxed);
    (value != usize::MAX).then_some(value)
}

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageSamplerDescriptor, ImageType};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Frames a second a video skin plays at, and how many its sheet holds.
const VIDEO_FPS: f32 = 8.0;
const VIDEO_FRAMES: usize = 16;
const VIDEO_TILE: usize = 128;
const VIDEO_GRID: usize = 4;

#[derive(Default)]
pub(crate) struct Painted {
    shown: Option<usize>,
    handle: Option<Handle<Image>>,
    /// The sheet of a video skin, as RGBA8 rows.
    sheet: Vec<u8>,
    frame: usize,
    started: f32,
    loaded: bool,
    published: Option<render_gpu::drawsurf::SkinPatch>,
}

fn decode(png: &[u8]) -> Option<Image> {
    let image = Image::from_buffer(
        png,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::Default,
        RenderAssetUsages::default(),
    )
    .ok()?;
    (image.texture_descriptor.format == TextureFormat::Rgba8UnormSrgb).then_some(image)
}

fn repeating() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        address_mode_w: bevy::image::ImageAddressMode::Repeat,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        min_filter: bevy::image::ImageFilterMode::Linear,
        ..default()
    })
}

/// Copy one 128x128 frame of the sheet into `out`.
fn copy_frame(sheet: &[u8], frame: usize, out: &mut [u8]) {
    let (cx, cy) = (frame % VIDEO_GRID, frame / VIDEO_GRID);
    let sheet_stride = VIDEO_TILE * VIDEO_GRID * 4;
    let row_bytes = VIDEO_TILE * 4;
    for row in 0..VIDEO_TILE {
        let from = (cy * VIDEO_TILE + row) * sheet_stride + cx * row_bytes;
        out[row * row_bytes..(row + 1) * row_bytes].copy_from_slice(&sheet[from..from + row_bytes]);
    }
}

/// Keep the viewmodel's skin what the menu chose: decode a new choice once,
/// and for a video step to its next frame when it is due.
pub(crate) fn paint(
    time: Res<Time<Real>>,
    plan: Option<Res<render_anim::FpvDrawPlan>>,
    mut images: ResMut<Assets<Image>>,
    mut painted: Local<Painted>,
) {
    // The skin chosen last stays until another is chosen, across matches and
    // launches. (`IW4L_SKIN`, a skin number, overrides it for tests.)
    if !painted.loaded {
        painted.loaded = true;
        let saved = crate::menu::read_setting(crate::menu::Setting::Skin);
        select_unsaved(
            std::env::var("IW4L_SKIN")
                .ok()
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(saved),
        );
    }
    let chosen = selected();
    if chosen != painted.shown {
        painted.shown = chosen;
        painted.sheet.clear();
        painted.frame = usize::MAX;
        painted.started = time.elapsed_secs();
        painted.handle = chosen.and_then(|index| {
            let source = decode(SKINS[index].png)?;
            let data = source.data.clone()?;
            if SKINS[index].video {
                painted.sheet = data;
                let mut first = vec![0u8; VIDEO_TILE * VIDEO_TILE * 4];
                copy_frame(&painted.sheet, 0, &mut first);
                painted.frame = 0;
                let mut image = Image::new(
                    Extent3d {
                        width: VIDEO_TILE as u32,
                        height: VIDEO_TILE as u32,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    first,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::default(),
                );
                image.sampler = repeating();
                Some(images.add(image))
            } else {
                let mut image = source;
                image.sampler = repeating();
                Some(images.add(image))
            }
        });
        diag::info!(
            Console,
            "skin: {:?} -> {}",
            chosen,
            if painted.handle.is_some() { "painted" } else { "none" }
        );
    }
    // Tell the renderer which colour maps to paint, and with what: the gun can
    // change (a new class, a swap) without the choice changing.
    let targets: Vec<_> = plan
        .as_ref()
        .map(|plan| plan.gun_colours().iter().map(|handle| handle.id()).collect())
        .unwrap_or_default();
    let patch = render_gpu::drawsurf::SkinPatch {
        targets,
        skin: painted.handle.as_ref().map(|handle| handle.id()),
    };
    if painted.published.as_ref() != Some(&patch) {
        render_gpu::drawsurf::set_skin_patch(patch.clone());
        painted.published = Some(patch);
    }
    let (Some(index), Some(handle)) = (chosen, painted.handle.clone()) else {
        return;
    };
    if !SKINS[index].video {
        return;
    }
    let frame = (((time.elapsed_secs() - painted.started) * VIDEO_FPS) as usize) % VIDEO_FRAMES;
    if frame == painted.frame {
        return;
    }
    painted.frame = frame;
    if let Some(mut image) = images.get_mut(&handle)
        && let Some(data) = image.data.as_mut()
    {
        copy_frame(&painted.sheet, frame, data);
    }
}
