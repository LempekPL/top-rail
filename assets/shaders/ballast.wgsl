#import bevy_sprite::mesh2d_vertex_output::VertexOutput

fn hash_2d(p: vec2<f32>) -> f32 {
    let n = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(n) * 43758.5453123);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = vec3<f32>(0.15, 0.15, 0.15);

    let noise_scale = 18.0;
    let noise = hash_2d(floor(in.uv * noise_scale)) * 0.15 - 0.075;
    color += noise;

    let dist_from_center = abs(in.uv.y - 0.5) * 2.0;
    let edge_shadow = 1.0 - (pow(dist_from_center, 3.0) * 0.6);

    color *= edge_shadow;

    return vec4<f32>(color, 1.0);
}