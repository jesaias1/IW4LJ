use std::sync::OnceLock;

use perfetto_sdk::track_event::{EventContext, TrackEventCounter, TrackEventTrack};

perfetto_sdk::track_event_categories! {
    pub(crate) mod vocabulary_te_ns {
        ("iw4l.frame", "IW4L iw4l.frame events", ["iw4l"]),
        ("iw4l.fx", "IW4L iw4l.fx events", ["iw4l"]),
        ("iw4l.render", "IW4L iw4l.render events", ["iw4l"]),
        ("iw4l.sim", "IW4L iw4l.sim events", ["iw4l"]),
    }
}

pub(crate) use vocabulary_te_ns as perfetto_te_ns;

pub(crate) fn register_categories() {
    let _ = vocabulary_te_ns::register();
}

pub use crate::vocabulary_types::Span;

const SPAN_COUNT: usize = Span::COUNT;
static SPAN_TRACKS: [OnceLock<TrackEventTrack>; SPAN_COUNT] =
    [const { OnceLock::new() }; SPAN_COUNT];

impl Span {
    fn track(self) -> &'static TrackEventTrack {
        SPAN_TRACKS[self as usize].get_or_init(|| {
            TrackEventTrack::register_named_track(
                self.track_name(),
                self as u64,
                TrackEventTrack::process_track_uuid(),
            )
            .expect("register Perfetto span track")
        })
    }

    fn track_name(self) -> &'static str {
        macro_rules! span_track {
            ($(
                $variant:ident {
                    ordinal: $ordinal:literal,
                    name: $name:literal,
                    track: $track:literal,
                    category: $category:literal,
                    coverage_root: $coverage_root:literal,
                },
            )*) => {{
                const _: () = {
                    $(
                        let _ = $ordinal;
                        let _ = $name;
                        let _ = $category;
                        let _ = $coverage_root;
                    )*
                };
                match self {
                    $(Self::$variant => $track,)*
                }
            }};
        }
        crate::vocabulary_catalog::spans!(span_track)
    }

    #[inline]
    pub fn begin(self) {
        crate::stats::begin(self);
        macro_rules! begin {
            ($category:tt, $name:tt) => {
                perfetto_sdk::track_event_begin!($category, $name, |ctx: &mut EventContext| {
                    ctx.set_track(self.track());
                })
            };
        }
        macro_rules! span_begin {
            ($(
                $variant:ident {
                    ordinal: $ordinal:literal,
                    name: $name:tt,
                    track: $track:tt,
                    category: $category:tt,
                    coverage_root: $coverage_root:literal,
                },
            )*) => {{
                const _: () = {
                    $(
                        let _ = $ordinal;
                        let _ = $track;
                        let _ = $coverage_root;
                    )*
                };
                match self {
                    $(Self::$variant => begin!($category, $name),)*
                }
            }};
        }
        crate::vocabulary_catalog::spans!(span_begin);
    }
    #[inline]
    pub fn end(self) {
        crate::stats::end(self);
        macro_rules! end {
            ($category:tt) => {
                perfetto_sdk::track_event_end!($category, |ctx: &mut EventContext| {
                    ctx.set_track(self.track());
                })
            };
        }
        macro_rules! span_end {
            ($(
                $variant:ident {
                    ordinal: $ordinal:literal,
                    name: $name:tt,
                    track: $track:tt,
                    category: $category:tt,
                    coverage_root: $coverage_root:literal,
                },
            )*) => {{
                const _: () = {
                    $(
                        let _ = $ordinal;
                        let _ = $name;
                        let _ = $track;
                        let _ = $coverage_root;
                    )*
                };
                match self {
                    $(Self::$variant => end!($category),)*
                }
            }};
        }
        crate::vocabulary_catalog::spans!(span_end);
    }
    #[inline]
    pub fn enter(self) -> SpanGuard {
        self.begin();
        SpanGuard { span: self }
    }
}

#[must_use = "the span closes when this guard drops"]
pub struct SpanGuard {
    span: Span,
}

impl Drop for SpanGuard {
    #[inline]
    fn drop(&mut self) {
        self.span.end();
    }
}

pub use crate::vocabulary_types::Counter;

const COUNTER_COUNT: usize = Counter::COUNT;
static COUNTER_TRACKS: [OnceLock<TrackEventTrack>; COUNTER_COUNT] =
    [const { OnceLock::new() }; COUNTER_COUNT];

impl Counter {
    fn track_name(self) -> &'static str {
        self.name()
    }

    fn track(self) -> &'static TrackEventTrack {
        COUNTER_TRACKS[self as usize].get_or_init(|| {
            TrackEventTrack::register_counter_track(
                self.track_name(),
                TrackEventTrack::process_track_uuid(),
            )
            .expect("register Perfetto counter track")
        })
    }

    #[inline]
    /// Emit a value measured inside a frame that has already closed.
    ///
    /// `frame` is [`crate::frames::open_index`] as it was when the work
    /// started. The Perfetto counter track is the same either way — it carries
    /// its own timestamp — and only the frame row moves.
    pub fn emit_at(self, value: f64, frame: u64) {
        crate::stats::count_at(self, value, frame);
        self.emit_track(value);
    }

    #[inline]
    pub fn emit(self, value: f64) {
        crate::stats::count(self, value);
        self.emit_track(value);
    }

    #[inline]
    fn emit_track(self, value: f64) {
        macro_rules! emit {
            ($category:tt) => {{
                if !perfetto_sdk::track_event_category_enabled!($category) {
                    return;
                }
                let track = self.track();
                perfetto_sdk::track_event_counter!($category, |ctx: &mut EventContext| {
                    ctx.set_track(track);
                    ctx.set_counter(TrackEventCounter::Double(value));
                })
            }};
        }
        macro_rules! counter_emit {
            ($(
                $(#[$attr:meta])*
                $variant:ident {
                    ordinal: $ordinal:literal,
                    name: $name:tt,
                    unit: $unit:ident,
                    unit_class: $unit_class:ident,
                    origin: $origin:ident,
                    origin_class: $origin_class:ident,
                    category: $category:tt,
                    category_class: $category_class:ident,
                },
            )*) => {{
                const _: () = {
                    $(
                        $(
                            let _ = stringify!($attr);
                        )*
                        let _ = $ordinal;
                        let _ = $name;
                        let _ = stringify!($unit);
                        let _ = stringify!($unit_class);
                        let _ = stringify!($origin);
                        let _ = stringify!($origin_class);
                        let _ = stringify!($category_class);
                    )*
                };
                match self {
                    $(Self::$variant => emit!($category),)*
                }
            }};
        }
        crate::vocabulary_catalog::counters!(counter_emit);
    }
}
