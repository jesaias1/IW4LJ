use asset_iw4::attenuate;

pub(crate) fn distance_attenuation(
    knots: &[[f32; 2]],
    near: Option<&[[f32; 2]]>,
    dist: f32,
    min: f32,
    max: f32,
) -> f32 {
    let Some(near) = near else {
        return attenuate(knots, dist, min, max);
    };
    let (knots, fraction) = if dist < min {
        (near, dist / min)
    } else {
        (
            knots,
            if max > min {
                (dist - min) / (max - min)
            } else {
                1.0
            },
        )
    };
    let Some(first) = knots.first() else {
        return -1.0;
    };
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction <= first[0] {
        return first[1];
    }
    for pair in knots.windows(2) {
        let [a, b] = pair else { unreachable!() };
        if fraction <= b[0] && b[0] > a[0] {
            return a[1] + (b[1] - a[1]) * (fraction - a[0]) / (b[0] - a[0]);
        }
    }
    knots.last().map_or(-1.0, |last| last[1])
}
