use bevy::asset::RenderAssetUsages;
use bevy::math::Vec2;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

pub mod bezier {
    use bevy::math::Vec2;
    use bevy::math::cubic_splines::CubicSegment;

    #[inline]
    pub fn build_segment(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2) -> CubicSegment<Vec2> {
        CubicSegment::new_bezier([p0, p1, p2, p3])
    }

    pub fn find_t_from_pos(segment: &CubicSegment<Vec2>, pos: Vec2) -> f32 {
        let mut t = 0.5;
        let iterations = 8;
        for _ in 0..iterations {
            let current_pos = segment.position(t);
            let d1 = segment.velocity(t);
            let d2 = segment.acceleration(t);
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
        let segment = build_segment(p0, p1, p2, p3);
        let t = find_t_from_pos(&segment, pos);
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
    let segment = bezier::build_segment(p0, p1, p2, p3);
    for i in 0..num_splits {
        let is_last = i == num_splits - 1;

        let ta = i as f32 * step;
        let tb = if is_last { 1.0 } else { (i + 1) as f32 * step };

        let q0 = last_q3;
        let q3 = if is_last { p3 } else { segment.position(tb) };

        let q1 = q0 + segment.velocity(ta) * (step / 3.0);
        let q2 = q3 - segment.velocity(tb) * (step / 3.0);

        segments.push((q0, q1, q2, q3));

        last_q3 = q3;
    }
    segments
}

#[derive(Default, Debug, Clone)]
pub struct MeshBuffer {
    pos: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    ind: Vec<u32>,
}

impl MeshBuffer {
    pub fn with_capacity(quad_count: usize) -> Self {
        Self {
            pos: Vec::with_capacity(quad_count * 4),
            uvs: Vec::with_capacity(quad_count * 4),
            ind: Vec::with_capacity(quad_count * 6),
        }
    }

    pub fn to_mesh(self) -> Mesh {
        let mut m = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.pos);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        m.insert_indices(Indices::U32(self.ind));
        m
    }

    pub fn push_quad(&mut self, bl: Vec2, br: Vec2, tr: Vec2, tl: Vec2) {
        self.push_quad_uv(bl, br, tr, tl, [0., 0.], [0., 1.], [1., 1.], [1., 0.]);
    }

    pub fn push_quad_uv(
        &mut self,
        bl: Vec2,
        br: Vec2,
        tr: Vec2,
        tl: Vec2,
        uv_bl: [f32; 2],
        uv_br: [f32; 2],
        uv_tr: [f32; 2],
        uv_tl: [f32; 2],
    ) {
        let start = self.pos.len() as u32;

        self.pos.extend_from_slice(&[
            [bl.x, bl.y, 0.],
            [br.x, br.y, 0.],
            [tr.x, tr.y, 0.],
            [tl.x, tl.y, 0.],
        ]);

        self.uvs.extend_from_slice(&[uv_bl, uv_br, uv_tr, uv_tl]);

        self.ind
            .extend_from_slice(&[start, start + 1, start + 2, start + 2, start + 3, start]);
    }
}
