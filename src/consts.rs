pub const PIXELS_PER_SEGMENT: f32 = 12.0;

pub mod camera {
    pub const ZOOM_SPEED: f32 = 0.25;
    pub const ZOOM_MIN: f32 = ZOOM_SPEED;
    pub const ZOOM_MAX: f32 = ZOOM_SPEED * (4. + 10.);
}

pub mod track {
    pub const TRACK_WIDTH: f32 = 16.0;
    pub const RAIL_OFFSET: f32 = 7.0;
    pub const RAIL_WIDTH: f32 = 1.0;
    pub const SLEEPER_SPACING: f32 = 15.0;
    pub const SLEEPER_WIDTH: f32 = 12.0;
    pub const SLEEPER_HEIGHT: f32 = 1.5;
}

pub mod building {
    pub const TRACK_BUILD_SNAP_RADIUS: f32 = 16.0;
    pub const BUILDING_SNAP_RADIUS: f32 = 16.0;
    pub const MIN_CURVATURE: f32 = 160.0;
    pub const MIN_LENGTH: f32 = 20.0;
    pub const SEGMENT_LENGTH: f32 = 160.0;
}
