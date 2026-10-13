macro_rules! last_variant {
    ($ty:ident, $head:ident, $($tail:ident),+ $(,)?) => {
        last_variant!($ty, $($tail),+)
    };
    ($ty:ident, $last:ident $(,)?) => {
        $ty::$last
    };
}

macro_rules! check_unit_class {
    (default, Count) => {};
    (explicit, $unit:ident) => {};
}

macro_rules! check_origin_class {
    (default, Cpu) => {};
    (explicit, $origin:ident) => {};
}

macro_rules! check_category_class {
    (default, $category:tt) => {
        check_default_category!($category);
    };
    (explicit, $category:tt) => {};
}

macro_rules! check_default_category {
    ("iw4l.render") => {};
}

macro_rules! nonempty_tt {
    ("") => {
        compile_error!("empty catalog literal");
    };
    ($other:tt) => {};
}

macro_rules! define_spans {
    ($(
        $variant:ident {
            ordinal: $ordinal:literal,
            name: $name:literal,
            track: $track:literal,
            category: $category:literal,
            coverage_root: $coverage_root:literal,
        },
    )*) => {
        #[repr(usize)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Span {
            $($variant,)*
        }

        impl Span {
            pub const COUNT: usize = last_variant!(Span, $($variant),*) as usize + 1;

            /// Every span, in declaration order. The report walks this; the recorder
            /// indexes its arrays by `span as usize`, so the two must not drift.
            pub const ALL: [Self; Self::COUNT] = [$(Self::$variant,)*];

            /// The name the span carries in a trace and in the bench report. It is the
            /// Perfetto event name without the `span.` track prefix.
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)*
                }
            }

            /// Whether this span is a top-level scope of the frame, declared here
            /// rather than observed.
            ///
            /// The frame's coverage is the union of these, clipped to the wall, and
            /// `wall - covered` is the time nothing accounted for. Reading "was
            /// anything else open under it on this thread" instead would make the
            /// answer depend on which worker the executor ran a `begin` and its `end`
            /// on: `PreUpdate` opens in `First` and closes in `RunFixedMainLoop`, and
            /// when those land on different threads the span is root on neither — its
            /// whole interval falls out of the union and the remainder counts it as
            /// unclassified.
            ///
            /// The four schedule spans do not overlap each other and the render
            /// thread's does not nest in any of them, so their union is the frame's
            /// covered time whatever thread each was observed on.
            pub const fn coverage_root(self) -> bool {
                match self {
                    $(Self::$variant => $coverage_root,)*
                }
            }
        }

        const _: () = {
            $(
                if $ordinal != Span::$variant as usize {
                    panic!("span ordinal drifted");
                }
                if $name.is_empty() || $track.is_empty() || $category.is_empty() {
                    panic!("span literal empty");
                }
            )*
            let count = [$($ordinal),*].len();
            if Span::COUNT != count {
                panic!("span count drifted");
            }
        };
    };
}

crate::vocabulary_catalog::spans!(define_spans);

/// What a counter's value means, so the report can label it and refuse to add
/// milliseconds to draw calls. A counter is one or the other for its whole
/// life; there is no counter whose unit depends on the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    /// A duration, emitted in milliseconds.
    Milliseconds,
    /// A count of things that happened in the frame the sample belongs to.
    Count,
}

macro_rules! define_counters {
    ($(
        $(#[$attr:meta])*
        $variant:ident {
            ordinal: $ordinal:literal,
            name: $name:literal,
            unit: $unit:ident,
            unit_class: $unit_class:ident,
            origin: $origin:ident,
            origin_class: $origin_class:ident,
            category: $category:tt,
            category_class: $category_class:ident,
        },
    )*) => {
        #[repr(usize)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Counter {
            $(
                $(#[$attr])*
                $variant,
            )*
        }

        impl Counter {
            pub const COUNT: usize = last_variant!(Counter, $($variant),*) as usize + 1;

            /// Every counter, in declaration order. The recorder indexes its arrays by
            /// `counter as usize`, so this and the enum must not drift.
            pub const ALL: [Self; Self::COUNT] = [$(Self::$variant,)*];

            /// The name the counter carries in a trace and in the bench report.
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)*
                }
            }

            /// The unit the emitter passes. `emit` takes an `f64` either way, so this
            /// is the only thing that tells a millisecond from a draw call.
            pub const fn unit(self) -> Unit {
                match self {
                    $(Self::$variant => Unit::$unit,)*
                }
            }

            /// Where the number was measured. A GPU counter is a timestamp the driver
            /// resolved some frames after the CPU one next to it, which is why the
            /// report never puts the two in the same total.
            pub const fn origin(self) -> Origin {
                match self {
                    $(Self::$variant => Origin::$origin,)*
                }
            }
        }

        const _: () = {
            $(
                if $ordinal != Counter::$variant as usize {
                    panic!("counter ordinal drifted");
                }
                if $name.is_empty() {
                    panic!("counter literal empty");
                }
                nonempty_tt!($category);
                check_unit_class!($unit_class, $unit);
                check_origin_class!($origin_class, $origin);
                check_category_class!($category_class, $category);
            )*
            let count = [$($ordinal),*].len();
            if Counter::COUNT != count {
                panic!("counter count drifted");
            }
        };
    };
}

crate::vocabulary_catalog::counters!(define_counters);

/// Which side of the device a counter was measured on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Cpu,
    Gpu,
}
