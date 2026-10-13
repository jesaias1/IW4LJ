macro_rules! scalar_get {
    (bool, $dvars:ident, $($field:ident).+) => {
        u8::from($dvars.$($field).+) as f32
    };
    (float, $dvars:ident, $($field:ident).+) => {
        $dvars.$($field).+
    };
}
pub(crate) use scalar_get;

macro_rules! scalar_set {
    (bool, $dvars:ident, $($field:ident).+, $value:expr) => {
        $dvars.$($field).+ = $value != 0.0
    };
    (float, $dvars:ident, $($field:ident).+, $value:expr) => {
        $dvars.$($field).+ = $value
    };
}
pub(crate) use scalar_set;

macro_rules! scalar_is_bool {
    (bool) => {
        true
    };
    (float) => {
        false
    };
}
pub(crate) use scalar_is_bool;

macro_rules! debug_scalars {
    (
        $ty:ident;
        $(
            $name:literal {
                path: [$($field:ident).+],
                min: $min:literal,
                max: $max:literal,
                kind: $kind:ident
            }
        ),+ $(,)?
    ) => {
        const NAMES: &[&str] = &[$($name,)+];

        fn scalar_current(dvars: &$ty, setting: &str) -> (f32, f32, f32, bool) {
            match setting {
                $(
                    $name => (
                        $crate::debug_scalar::scalar_get!($kind, dvars, $($field).+),
                        $min,
                        $max,
                        $crate::debug_scalar::scalar_is_bool!($kind),
                    ),
                )+
                _ => unreachable!(),
            }
        }

        fn scalar_assign(dvars: &mut $ty, setting: &str, value: f32) {
            match setting {
                $(
                    $name => {
                        $crate::debug_scalar::scalar_set!($kind, dvars, $($field).+, value);
                    }
                )+
                _ => unreachable!(),
            }
        }
    };
}
pub(crate) use debug_scalars;
