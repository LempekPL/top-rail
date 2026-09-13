use bevy::color::Color;
use bevy::math::Vec2;
use bevy::prelude::Gizmos;

pub fn eval_bezier(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let u = 1.0 - t;
    p0 * (u * u * u) + p1 * (3.0 * u * u * t) + p2 * (3.0 * u * t * t) + p3 * (t * t * t)
}

pub fn eval_derivative(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let u = 1.0 - t;
    3.0 * u * u * (p1 - p0) + 6.0 * u * t * (p2 - p1) + 3.0 * t * t * (p3 - p2)
}

pub fn get_sweep(start: f32, end: f32, is_cw: bool) -> f32 {
    let mut sweep = end - start;
    if is_cw && sweep > 0.0 { sweep -= std::f32::consts::TAU; }
    if !is_cw && sweep < 0.0 { sweep += std::f32::consts::TAU; }
    sweep
}

pub fn generate_arc(c: Vec2, r: f32, p_start: Vec2, p_end: Vec2, is_cw: bool, segment_length: f32) -> Vec<(Vec2, Vec2, Vec2, Vec2)> {
    let mut segments = Vec::new();
    let start_angle = (p_start - c).to_angle();
    let end_angle = (p_end - c).to_angle();
    let sweep = get_sweep(start_angle, end_angle, is_cw);

    if sweep.abs() < 0.01 { return segments; }

    let arc_length = r * sweep.abs();
    let splits_by_length = (arc_length / segment_length).ceil() as usize;
    let splits_by_angle = (sweep.abs() / std::f32::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let num_splits = splits_by_length.max(splits_by_angle).max(1);
    let step = sweep / num_splits as f32;
    let k = (4.0 / 3.0) * (step.abs() / 4.0).tan();
    let turn_dir = sweep.signum();

    let mut current_angle = start_angle;
    for _ in 0..num_splits {
        let next_angle = current_angle + step;
        let tan0 = Vec2::new(-current_angle.sin(), current_angle.cos()) * turn_dir;
        let tan1 = Vec2::new(-next_angle.sin(), next_angle.cos()) * turn_dir;

        let q0 = c + Vec2::new(current_angle.cos(), current_angle.sin()) * r;
        let q3 = c + Vec2::new(next_angle.cos(), next_angle.sin()) * r;

        let q1 = q0 + tan0 * (k * r);
        let q2 = q3 - tan1 * (k * r);

        segments.push((q0, q1, q2, q3));
        current_angle = next_angle;
    }
    segments
}

pub fn generate_straight(dir: Vec2, dist: f32, p_start: Vec2, segment_length: f32) -> Vec<(Vec2, Vec2, Vec2, Vec2)> {
    let num_splits = (dist / segment_length).ceil().max(1.0) as usize;
    let step_len = dist / num_splits as f32;
    let mut segments = Vec::new();
    for i in 0..num_splits {
        let q0 = p_start + dir * (i as f32 * step_len);
        let q3 = p_start + dir * ((i + 1) as f32 * step_len);
        segments.push((q0, q0.lerp(q3, 0.33), q0.lerp(q3, 0.66), q3));
    }
    segments
}

pub fn calculate_path(
    p0: Vec2, t0: Vec2, p3: Vec2, t3: Vec2, r: f32, segment_length: f32
) -> Vec<(Vec2, Vec2, Vec2, Vec2)> {
    let n0_l = Vec2::new(-t0.y, t0.x);
    let n0_r = Vec2::new(t0.y, -t0.x);
    let n3_l = Vec2::new(-t3.y, t3.x);
    let n3_r = Vec2::new(t3.y, -t3.x);

    let c0_l = p0 + n0_l * r;
    let c0_r = p0 + n0_r * r;
    let c3_l = p3 + n3_l * r;
    let c3_r = p3 + n3_r * r;

    let paths = [
        (c0_l, c3_l, false, false),
        (c0_r, c3_r, true, true),
        (c0_l, c3_r, false, true),
        (c0_r, c3_l, true, false),
    ];

    let mut best_path = None;
    let mut min_length = f32::MAX;
    let mut best_pts = (Vec2::ZERO, Vec2::ZERO);

    for (c0, c3, cw0, cw3) in paths {
        let v = c3 - c0;
        let dist = v.length();
        let angle = v.to_angle();

        let mut pt1 = Vec2::ZERO;
        let mut pt2 = Vec2::ZERO;
        let mut valid = true;

        if cw0 == cw3 {
            let offset = if cw0 { std::f32::consts::FRAC_PI_2 } else { -std::f32::consts::FRAC_PI_2 };
            let dir = Vec2::new((angle + offset).cos(), (angle + offset).sin());
            pt1 = c0 + dir * r;
            pt2 = c3 + dir * r;
        } else {
            if dist < r * 2.0 {
                valid = false;
            } else {
                let theta = (r * 2.0 / dist).acos();
                let offset = if cw0 { theta } else { -theta };
                pt1 = c0 + Vec2::new((angle + offset).cos(), (angle + offset).sin()) * r;
                pt2 = c3 + Vec2::new((angle + offset + std::f32::consts::PI).cos(), (angle + offset + std::f32::consts::PI).sin()) * r;
            }
        }

        if valid {
            let sweep0 = get_sweep((p0 - c0).to_angle(), (pt1 - c0).to_angle(), cw0);
            let sweep3 = get_sweep((pt2 - c3).to_angle(), (p3 - c3).to_angle(), cw3);
            let total_len = r * sweep0.abs() + pt1.distance(pt2) + r * sweep3.abs();

            if total_len < min_length {
                min_length = total_len;
                best_path = Some((c0, c3, cw0, cw3));
                best_pts = (pt1, pt2);
            }
        }
    }

    let mut segments = Vec::new();
    if let Some((c0, c3, cw0, cw3)) = best_path {
        let (pt1, pt2) = best_pts;
        segments.extend(generate_arc(c0, r, p0, pt1, cw0, segment_length));
        let line_dist = pt1.distance(pt2);
        if line_dist > 1.0 {
            let dir = (pt2 - pt1).normalize_or_zero();
            segments.extend(generate_straight(dir, line_dist, pt1, segment_length));
        }
        segments.extend(generate_arc(c3, r, pt2, p3, cw3, segment_length));
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
        let u = 1.0 - t;

        let current_point =
            p0 * (u * u * u) + p1 * (3.0 * u * u * t) + p2 * (3.0 * u * t * t) + p3 * (t * t * t);

        gizmos.line_2d(prev_point, current_point, color);
        prev_point = current_point;
    }
}
