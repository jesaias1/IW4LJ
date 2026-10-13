use std::collections::HashMap;
use std::time::Duration;

use asset_game::MenuCatalog;
use bevy::prelude::*;
use net::LastAdoptedSnapshot;
use net::transport::udp_session::UdpClientLink;

use crate::draw2d::{Draw2dCmd, Draw2dList, Draw2dOp, Draw2dProvenance, tessellate_fonts};
use crate::gpu_list::{HudTessPass, TessJob};
use crate::images::HUD_CHROME_NAMESPACE;
use crate::images::HudImages;

const SHOW_AFTER: Duration = Duration::from_millis(1000);

#[derive(Component)]
pub(crate) struct ConnectionNoticeRaster;

pub(crate) fn update(
    surface: Res<crate::surface::Hud2dSurface>,
    catalog: Option<Res<MenuCatalog>>,
    link: Option<Res<UdpClientLink>>,
    adopted: Option<Res<LastAdoptedSnapshot>>,
    mut pass: ResMut<HudTessPass>,
    mut hud_images: ResMut<HudImages>,
    mut images: ResMut<Assets<Image>>,
) {
    pass.connection_notice = TessJob::Hide;
    if !surface.is_ready() || !link.is_some_and(|link| link.has_entered_match()) {
        return;
    }
    let Some(silent) = adopted
        .and_then(|adopted| adopted.since_adopted())
        .filter(|d| *d >= SHOW_AFTER)
    else {
        return;
    };
    let Some(catalog) = catalog.as_ref() else {
        return;
    };
    let Some(item) = catalog
        .get("hud_fullscreen")
        .and_then(|m| m.items.iter().find(|i| i.owner_draw == 113))
    else {
        return;
    };
    let font_name = hud_iw4::ui_get_font_handle(
        item.font_enum,
        surface.scale_virtual_to_real()[1],
        item.text_scale,
    );
    let Some(font) = catalog.font(font_name) else {
        return;
    };
    let text = format!("Connection interrupted  {:.1}s", silent.as_secs_f32());
    let nscale = hud_iw4::normalized_text_scale(font.pixel_height, item.text_scale);
    let material = asset_core::AssetRef::bare_name(&font.material).to_owned();
    let width = crate::chrome::ui_text_width(font, &text, item.text_scale);
    let rect = surface.apply_rect(
        -width * 0.5,
        -120.0,
        nscale,
        nscale,
        hud_iw4::ALIGN_CENTER,
        hud_iw4::ALIGN_CENTER,
    );
    let list = Draw2dList {
        cmds: vec![Draw2dCmd {
            material_namespace: HUD_CHROME_NAMESPACE,
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
            s0: 0.0,
            t0: 0.0,
            s1: 1.0,
            t1: 1.0,
            color: [1.0, 0.3, 0.3, 1.0],
            material: material.clone(),
            op: Draw2dOp::TextRun {
                font: font_name.to_owned(),
                scale: nscale,
                text,
                loc_key: String::new(),
                style: item.text_style,
                fx: None,
                glow: None,
            },
            provenance: Draw2dProvenance::CgDraw {
                site: "connection_notice",
            },
            layer: 1,
        }],
    };
    let mut fonts = HashMap::new();
    fonts.insert(font_name.to_owned(), font);
    let (quads, _) = tessellate_fonts(&list, &fonts);
    if !quads.is_empty()
        && hud_images
            .get(HUD_CHROME_NAMESPACE, &material, &mut images)
            .is_some()
    {
        pass.connection_notice = TessJob::Quads(quads);
    }
}
