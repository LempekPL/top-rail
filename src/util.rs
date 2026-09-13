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
