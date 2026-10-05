use bevy::asset::RenderAssetUsages;
use bevy::math::Vec2;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::CubicSegment;

pub mod curve {
    use bevy::math::Vec2;
    use bevy::math::cubic_splines::CubicSegment;

    #[inline]
    pub fn build_segment(
        start_pos: Vec2,
        start_tangent: Vec2,
        end_pos: Vec2,
        end_tangent: Vec2,
    ) -> CubicSegment<Vec2> {
        CubicSegment {
            coeff: [
                start_pos,
                start_tangent,
                (end_pos - start_pos) * 3. - start_tangent * 2. - end_tangent,
                (start_pos - end_pos) * 2. + start_tangent + end_tangent,
            ],
        }
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
        segment: &CubicSegment<Vec2>,
        t: f32,
    ) -> (CubicSegment<Vec2>, CubicSegment<Vec2>) {
        let start_pos = segment.position(0.0);
        let start_tangent = segment.velocity(0.0);
        let end_pos = segment.position(1.0);
        let end_tangent = segment.velocity(1.0);

        let mid_pos = segment.position(t);
        let mid_tangent = segment.velocity(t);

        let left = build_segment(start_pos, start_tangent * t, mid_pos, mid_tangent * t);
        let right = build_segment(
            mid_pos,
            mid_tangent * (1.0 - t),
            end_pos,
            end_tangent * (1.0 - t),
        );

        (left, right)
    }

    pub fn split_at_pos(
        segment: &CubicSegment<Vec2>,
        pos: Vec2,
    ) -> (CubicSegment<Vec2>, CubicSegment<Vec2>) {
        let t = find_t_from_pos(segment, pos);
        split_at_t(segment, t)
    }
}

pub fn create_straight(
    start_pos: Vec2,
    end_pos: Vec2,
    segment_length: f32,
) -> Vec<CubicSegment<Vec2>> {
    let line = end_pos - start_pos;
    let dist = line.length();
    if dist < 0.001 {
        // ignore when short
        return Vec::new();
    }
    let dir = line / dist;
    let num_splits = (dist / segment_length).ceil().max(1.0) as usize;
    let step_len = dist / num_splits as f32;
    let mut segments = Vec::with_capacity(num_splits);
    let tangent = dir * step_len;
    let mut last_pos = start_pos;
    for i in 0..num_splits {
        let next_pos = if i == num_splits - 1 {
            end_pos
        } else {
            start_pos + dir * ((i + 1) as f32 * step_len)
        };
        segments.push(curve::build_segment(last_pos, tangent, next_pos, tangent));
        last_pos = next_pos;
    }

    segments
}

pub fn create_arc(
    start_pos: Vec2,
    start_tangent_dir: Vec2,
    end_pos: Vec2,
    segment_length: f32,
) -> Vec<CubicSegment<Vec2>> {
    let normal = Vec2::new(-start_tangent_dir.y, start_tangent_dir.x);
    let chord = end_pos - start_pos;
    let d = chord.dot(normal);

    // if chord and tangent are very close just make it straight
    if d.abs() < 0.00001 {
        return create_straight(start_pos, end_pos, segment_length);
    }

    let turn_dir = d.signum();
    let radius = chord.length_squared() / (2.0 * d.abs());
    let center = start_pos + normal * radius * turn_dir;

    let start_angle = (start_pos - center).to_angle();
    let end_angle = (end_pos - center).to_angle();
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
    let tangent_mag = radius * step.abs();

    let mut segments = Vec::with_capacity(num_splits);
    let mut current_angle = start_angle;
    let mut last_pos = start_pos;

    for i in 0..num_splits {
        let is_last = i == num_splits - 1;
        let next_angle = if is_last {
            end_angle
        } else {
            start_angle + (i + 1) as f32 * step
        };

        let t0 = Vec2::new(-current_angle.sin(), current_angle.cos()) * turn_dir * tangent_mag;
        let t1 = Vec2::new(-next_angle.sin(), next_angle.cos()) * turn_dir * tangent_mag;

        let next_pos = if is_last {
            end_pos
        } else {
            center + Vec2::from_angle(next_angle) * radius
        };

        segments.push(curve::build_segment(last_pos, t0, next_pos, t1));
        current_angle = next_angle;
        last_pos = next_pos;
    }
    segments
}

pub fn create_segmented_curve(
    start_point: Vec2,
    start_tangent_dir: Vec2,
    end_point: Vec2,
    end_tangent_dir: Vec2,
    segment_length: f32,
) -> Vec<CubicSegment<Vec2>> {
    let dist = start_point.distance(end_point);
    let d = dist * 1.35;

    let start_tangent = start_tangent_dir.normalize_or_zero() * d;
    let end_tangent = end_tangent_dir.normalize_or_zero() * d;
    let segment = curve::build_segment(start_point, start_tangent, end_point, end_tangent);

    let num_splits = (dist / segment_length).ceil().max(1.0) as usize;
    let step = 1.0 / num_splits as f32;

    let mut segments = Vec::with_capacity(num_splits);
    let mut last_pos = start_point;
    let mut last_tangent = segment.velocity(0.0) * step;
    for i in 0..num_splits {
        let is_last = i == num_splits - 1;
        let tb = if is_last { 1.0 } else { (i + 1) as f32 * step };
        let next_pos = if is_last {
            end_point
        } else {
            segment.position(tb)
        };
        let next_tangent = segment.velocity(tb) * step;

        segments.push(curve::build_segment(
            last_pos,
            last_tangent,
            next_pos,
            next_tangent,
        ));

        last_pos = next_pos;
        last_tangent = next_tangent;
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
        let colors = vec![[1.0, 1.0, 1.0, 1.0]; self.pos.len()];
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.pos);
        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
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
