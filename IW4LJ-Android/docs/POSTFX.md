# Post effects by map family

The map chooses its authored vision preset; weapon family does not choose the
screen grade. Explicit script visions and transitions override the map preset.

| Map family | Presented profile | Remaining native effects |
|---|---|---|
| IW4 | Authored film contrast, brightness, desaturation, tints and glow | Extra glow/sky-bleed file controls need PC verification |
| IW5 | Map/common vision film and glow through the shared film profile | Hero saturation and extra glow controls need verification |
| T5 | Shared script blur/DoF; native film and bloom are not selected | Three-band film, bloom curves, persistence and streaks |
| T6 | Three-band film, bloom levels/gamma/tints, two-scale blur and highlight compression | HQ bloom, LUT volumes, SSAO and special camera filters |

T6 bloom preserves authored RGB and luminance channels independently. Bloom
levels are filtered at quarter, eighth and sixteenth resolution, then composed
in linear color before highlight compression and film grading. Scene exposure
already supplies the display normalization. Bloom intermediates use floating
point textures to preserve values above one.

T6 map presets override common presets with the same name. Disabled film retains
bloom; a malformed preset retains its parse error. Missing controls are not
replaced with a generic contrast boost. Native presets transition their film,
bloom and highlight compression together. User bloom settings apply unless a
script forces the effect; explicit script disable clears bloom or film grading.

Screen blur, DoF and saved-screen feedback are triggered by gameplay and script
state. They are not an always-on softening filter. Film here means color grading;
there is no automatic grain layer. HUD draws after scene post processing.

See [GSC post effects](GSC-POSTFX.md) for script commands and dvars.
