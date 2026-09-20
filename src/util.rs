use bevy::color::Color;
use bevy::math::Vec2;
use bevy::prelude::Gizmos;

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

    // pub fn offset(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32, offset: f32) -> Vec2 {}
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
    let mut segments = Vec::new();
    for i in 0..num_splits {
        let q0 = p_start + dir * (i as f32 * step_len);
        let q3 = if i == num_splits - 1 {
            // make sure it ends on point
            p_end
        } else {
            p_start + dir * ((i + 1) as f32 * step_len)
        };
        segments.push((q0, q0.lerp(q3, 0.33), q0.lerp(q3, 0.66), q3));
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

    let mut segments = Vec::new();
    let mut current_angle = start_angle;
    for _ in 0..num_splits {
        let next_angle = current_angle + step;

        let t0 = Vec2::new(-current_angle.sin(), current_angle.cos()) * turn_dir;
        let t1 = Vec2::new(-next_angle.sin(), next_angle.cos()) * turn_dir;

        let q0 = center + Vec2::from_angle(current_angle) * radius;
        let q3 = center + Vec2::from_angle(next_angle) * radius;

        let q1 = q0 + t0 * (k * radius);
        let q2 = q3 - t1 * (k * radius);

        segments.push((q0, q1, q2, q3));
        current_angle = next_angle;
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
    for i in 0..num_splits {
        let ta = i as f32 * step;
        let tb = (i + 1) as f32 * step;

        let q0 = bezier::eval(p0, p1, p2, p3, ta);
        let q3 = bezier::eval(p0, p1, p2, p3, tb);

        let q1 = q0 + bezier::derivative(p0, p1, p2, p3, ta) * (step / 3.0);
        let q2 = q3 - bezier::derivative(p0, p1, p2, p3, tb) * (step / 3.0);

        segments.push((q0, q1, q2, q3));
    }
    segments
}

pub fn draw_bezier(gizmos: &mut Gizmos, p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, color: Color) {
    let pixels_per_segment = 15.0;

    let approx_length = p0.distance(p1) + p1.distance(p2) + p2.distance(p3);
    let calculated_segments = (approx_length / pixels_per_segment).ceil() as usize;
    let segments = calculated_segments.clamp(10, 256);
    let mut prev_point = p0;
    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let current_point = bezier::eval(p0, p1, p2, p3, t);
        gizmos.line_2d(prev_point, current_point, color);
        prev_point = current_point;
    }
}
