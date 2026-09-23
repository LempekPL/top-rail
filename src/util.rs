use bevy::color::Color;
use bevy::math::Vec2;
use bevy::prelude::{CubicBezier, CubicGenerator, Gizmos};

pub mod bezier {
    use bevy::math::Vec2;

    pub fn eval(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
        let u = 1.0 - t;
        p0 * (u * u * u) + p1 * (3.0 * u * u * t) + p2 * (3.0 * u * t * t) + p3 * (t * t * t)
    }

    pub fn derivative(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
        let u = 1.0 - t;
        3.0 * u * u * (p1 - p0) + 6.0 * u * t * (p2 - p1) + 3.0 * t * t * (p3 - p2)
    }

    pub fn second_derivative(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
        let u = 1.0 - t;
        6.0 * u * (p2 - p1 * 2.0 + p0) + 6.0 * t * (p3 - p2 * 2.0 + p1)
    }

    pub fn find_t_from_pos(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, pos: Vec2) -> f32 {
        let mut t = 0.5;
        let iterations = 8;
        for _ in 0..iterations {
            let current_pos = eval(p0, p1, p2, p3, t);
            let d1 = derivative(p0, p1, p2, p3, t);
            let d2 = second_derivative(p0, p1, p2, p3, t);
            let delta = current_pos - pos;
            let f = delta.dot(d1);
            let f_prime = d1.dot(d1) + delta.dot(d2);
            if f_prime.abs() < 1e-6 {
                break;
            }
            t -= f / f_prime;
        }
        t.clamp(0.0, 1.0)
    }

    pub fn split_at_t(
        p0: Vec2,
        p1: Vec2,
        p2: Vec2,
        p3: Vec2,
        t: f32,
    ) -> ((Vec2, Vec2, Vec2, Vec2), (Vec2, Vec2, Vec2, Vec2)) {
        let p01 = p0.lerp(p1, t);
        let p12 = p1.lerp(p2, t);
        let p23 = p2.lerp(p3, t);

        let p012 = p01.lerp(p12, t);
        let p123 = p12.lerp(p23, t);

        let p0123 = p012.lerp(p123, t);

        ((p0, p01, p012, p0123), (p0123, p123, p23, p3))
    }

    pub fn split_at_pos(
        p0: Vec2,
        p1: Vec2,
        p2: Vec2,
        p3: Vec2,
        pos: Vec2,
    ) -> ((Vec2, Vec2, Vec2, Vec2), (Vec2, Vec2, Vec2, Vec2)) {
        let t = find_t_from_pos(p0, p1, p2, p3, pos);
        split_at_t(p0, p1, p2, p3, t)
    }
}

pub fn create_straight(
    p_start: Vec2,
    p_end: Vec2,
    segment_length: f32,
) -> Vec<(Vec2, Vec2, Vec2, Vec2)> {
    let line = p_end - p_start;
    let dist = line.length();
    if dist < 0.001 {
        // ignore when short
        return Vec::new();
    }
    let dir = line / dist;
    let num_splits = (dist / segment_length).ceil().max(1.0) as usize;
    let step_len = dist / num_splits as f32;
    let mut segments = Vec::with_capacity(num_splits);
    let mut last_q3 = p_start;
    for i in 0..num_splits {
        let q0 = last_q3;
        let q3 = if i == num_splits - 1 {
            p_end
        } else {
            p_start + dir * ((i + 1) as f32 * step_len)
        };
        segments.push((q0, q0.lerp(q3, 1. / 3.), q0.lerp(q3, 2. / 3.), q3));
        last_q3 = q3;
    }

    segments
}

pub fn create_arc(
    p_start: Vec2,
    start_tangent: Vec2,
    p_end: Vec2,
    segment_length: f32,
) -> Vec<(Vec2, Vec2, Vec2, Vec2)> {
    let normal = Vec2::new(-start_tangent.y, start_tangent.x);
    let chord = p_end - p_start;
    let d = chord.dot(normal);

    // if chord and tangent are very close just make it straight
    if d.abs() < 0.00001 {
        return create_straight(p_start, p_end, segment_length);
    }

    let turn_dir = d.signum();
    let radius = chord.length_squared() / (2.0 * d.abs());
    let center = p_start + normal * radius * turn_dir;

    let start_angle = (p_start - center).to_angle();
    let end_angle = (p_end - center).to_angle();
    let mut sweep_angle = end_angle - start_angle;
    if turn_dir > 0.0 && sweep_angle < 0.0 {
        sweep_angle += std::f32::consts::TAU;
    }
    if turn_dir < 0.0 && sweep_angle > 0.0 {
        sweep_angle -= std::f32::consts::TAU;
    }
    let arc_length = radius * sweep_angle.abs();
    let num_splits = (arc_length / segment_length).ceil().max(1.0) as usize;

    let step = sweep_angle / num_splits as f32;
    let k = (4.0 / 3.0) * (step.abs() / 4.0).tan();

    let mut segments = Vec::with_capacity(num_splits);
    let mut current_angle = start_angle;
    let mut last_q3 = p_start;
    for i in 0..num_splits {
        let is_last = i == num_splits - 1;
        let next_angle = if is_last {
            end_angle
        } else {
            start_angle + (i + 1) as f32 * step
        };

        let t0 = Vec2::new(-current_angle.sin(), current_angle.cos()) * turn_dir;
        let t1 = Vec2::new(-next_angle.sin(), next_angle.cos()) * turn_dir;

        let q0 = last_q3;
        let q3 = if is_last {
            p_end
        } else {
            center + Vec2::from_angle(next_angle) * radius
        };

        let q1 = q0 + t0 * (k * radius);
        let q2 = q3 - t1 * (k * radius);

        segments.push((q0, q1, q2, q3));
        current_angle = next_angle;
        last_q3 = q3;
    }
    segments
}

pub fn create_segmented_bezier(
    p0: Vec2,
    t0: Vec2,
    p3: Vec2,
    t3: Vec2,
    segment_length: f32,
) -> Vec<(Vec2, Vec2, Vec2, Vec2)> {
    let dist = p0.distance(p3);
    let d = dist * 0.45;

    let p1 = p0 + t0 * d;
    let p2 = p3 - t3 * d;

    let num_splits = (dist / segment_length).ceil().max(1.0) as usize;
    let step = 1.0 / num_splits as f32;

    let mut segments = Vec::new();
    let mut last_q3 = p0;
    for i in 0..num_splits {
        let is_last = i == num_splits - 1;

        let ta = i as f32 * step;
        let tb = if is_last { 1.0 } else { (i + 1) as f32 * step };

        let q0 = last_q3;
        let q3 = if is_last {
            p3
        } else {
            bezier::eval(p0, p1, p2, p3, tb)
        };

        let q1 = q0 + bezier::derivative(p0, p1, p2, p3, ta) * (step / 3.0);
        let q2 = q3 - bezier::derivative(p0, p1, p2, p3, tb) * (step / 3.0);

        segments.push((q0, q1, q2, q3));

        last_q3 = q3;
    }
    segments
}

pub fn draw_bezier(gizmos: &mut Gizmos, p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, color: Color) {
    let pixels_per_segment = 15.0;

    let approx_length = p0.distance(p1) + p1.distance(p2) + p2.distance(p3);
    let calculated_segments = (approx_length / pixels_per_segment).ceil() as usize;
    let segments = calculated_segments.clamp(10, 256);
    gizmos.curve_2d(
        CubicBezier::new([[p0, p1, p2, p3]]).to_curve().unwrap(),
        (0..=segments).map(|n| n as f32 / segments as f32),
        color,
    );

    // let mut prev_point = p0;
    // for i in 1..=segments {
    //     let t = i as f32 / segments as f32;
    //     let current_point = bezier::eval(p0, p1, p2, p3, t);
    //     gizmos.line_2d(prev_point, current_point, color);
    //     prev_point = current_point;
    // }
}
