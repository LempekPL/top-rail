#import bevy_sprite::mesh2d_vertex_output::VertexOutput

fn perlinNoise2_permute4(x: vec4f) -> vec4f {
    return ((x * 34. + 1.) * x) % vec4f(289.);
}

fn perlinNoise2_fade2(t: vec2f) -> vec2f {
    return t * t * t * (t * (t * 6. - 15.) + 10.);
}

fn perlinNoise2(P: vec2f) -> f32 {
    var Pi: vec4f = floor(P.xyxy) + vec4f(0., 0., 1., 1.);
    let Pf = fract(P.xyxy) - vec4f(0., 0., 1., 1.);
    Pi = Pi % vec4f(289.); // To avoid truncation effects in permutation
    let ix = Pi.xzxz;
    let iy = Pi.yyww;
    let fx = Pf.xzxz;
    let fy = Pf.yyww;
    let i = perlinNoise2_permute4(perlinNoise2_permute4(ix) + iy);
    var gx: vec4f = 2. * fract(i * 0.0243902439) - 1.; // 1/41 = 0.024...
    let gy = abs(gx) - 0.5;
    let tx = floor(gx + 0.5);
    gx = gx - tx;
    var g00: vec2f = vec2f(gx.x, gy.x);
    var g10: vec2f = vec2f(gx.y, gy.y);
    var g01: vec2f = vec2f(gx.z, gy.z);
    var g11: vec2f = vec2f(gx.w, gy.w);
    let norm = 1.79284291400159 - 0.85373472095314 *
        vec4f(dot(g00, g00), dot(g01, g01), dot(g10, g10), dot(g11, g11));
    g00 = g00 * norm.x;
    g01 = g01 * norm.y;
    g10 = g10 * norm.z;
    g11 = g11 * norm.w;
    let n00 = dot(g00, vec2f(fx.x, fy.x));
    let n10 = dot(g10, vec2f(fx.y, fy.y));
    let n01 = dot(g01, vec2f(fx.z, fy.z));
    let n11 = dot(g11, vec2f(fx.w, fy.w));
    let fade_xy = perlinNoise2_fade2(Pf.xy);
    let n_x = mix(vec2f(n00, n01), vec2f(n10, n11), vec2f(fade_xy.x));
    let n_xy = mix(n_x.x, n_x.y, fade_xy.y);
    return 2.3 * n_xy;
}

fn hash_2d(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453123);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let world_pos = (in.uv - 0.5) * 100000.0;
    let n = perlinNoise2(world_pos * 0.001);

    let threshold = 0.10;
    let smoothness = 1.0;
    let step_n = smoothstep(threshold - smoothness, threshold + smoothness, n);
    let color_grass = vec3<f32>(0.03, 0.14, 0.01);
    let color_grass_dark = vec3<f32>(0.01, 0.08, 0);
    let grass_color = mix(color_grass, color_grass_dark, n);

//    let color_greener = vec3<f32>(0.001, 0.07, 0);
//    let grainy = perlinNoise2(world_pos * 0.1);
//    let grainy_threshold = 0.60;
//    let grainy_smoothness = 1.;
//    let step_grainy = (smoothstep(grainy_threshold - grainy_smoothness, grainy_threshold + grainy_smoothness, grainy) + 1.) / 2.;
//    let final_color = mix(grass_color, color_greener, step_grainy);
//    let final_color = vec3<f32>(step_grainy);
    return vec4<f32>(grass_color, 1.0);
}