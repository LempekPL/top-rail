use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::{
    Slider, SliderDragState, SliderOrientation, SliderRange, SliderThumb, SliderValue, TrackClick,
    ValueChange,
};

#[derive(SceneComponent, Clone, Debug)]
#[scene(LeverSliderProps)]
pub struct LeverSlider {
    pub thumb_color: Color,
}

impl Default for LeverSlider {
    fn default() -> Self {
        Self {
            thumb_color: LeverSliderProps::default().thumb_color,
        }
    }
}

pub struct LeverSliderProps {
    pub min: f32,
    pub max: f32,
    pub value: f32,
    pub color: Color,
    pub thumb_color: Color,
}

impl Default for LeverSliderProps {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            value: 0.0,
            color: Color::srgb(0.2, 0.2, 0.2),
            thumb_color: Color::srgb(0.65, 0.65, 0.65),
        }
    }
}

impl LeverSlider {
    fn scene(props: LeverSliderProps) -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                height: percent(80),
                width: px(12),
            }
            LeverSlider {
                thumb_color: {props.thumb_color}
            }
            Slider {
                track_click: TrackClick::Drag,
                orientation: SliderOrientation::Vertical,
            }
            SliderValue({props.value})
            SliderRange::new(props.min, props.max)
            Hovered
            TabIndex(0)
            Children [
                (
                    Node {
                        width: px(8),
                        height: percent(100),
                        border_radius: BorderRadius::all(px(4)),
                        margin: UiRect::all(px(16)),
                    }
                    BackgroundColor({props.color})
                ),

                (
                    Node {
                        display: Display::Flex,
                        position_type: PositionType::Absolute,
                        left: px(38),
                        right: px(0),
                        top: px(32),
                        bottom: px(0),
                    }
                    Children [
                        SliderThumb
                        Node {
                            display: Display::Flex,
                            width: px(64),
                            height: px(32),
                            right: px(0),
                            position_type: PositionType::Absolute,
                            bottom: percent(0),
                            border_radius: BorderRadius::all(px(8)),
                        }
                        BackgroundColor({props.color})
                    ]
                ),
            ]
            on(|value_change: On<ValueChange<f32>>, mut commands: Commands| {
                commands.entity(value_change.source).insert(SliderValue(value_change.value));
            })
        }
    }
}

pub fn update_slider_style(
    sliders: Query<
        (
            Entity,
            Option<&Hovered>,
            Option<&SliderDragState>,
            &LeverSlider,
        ),
        With<Slider>,
    >,
    children: Query<&Children>,
    mut thumbs: Query<(&mut BackgroundColor, Has<SliderThumb>), Without<Slider>>,
) {
    for (slider_ent, hovered, drag_state, lever) in sliders.iter() {
        let is_hovered = hovered.map_or(false, |h| h.0);
        let is_dragging = drag_state.map_or(false, |d| d.dragging);

        for child in children.iter_descendants(slider_ent) {
            if let Ok((mut thumb_bg, is_thumb)) = thumbs.get_mut(child) {
                if is_thumb {
                    thumb_bg.0 = if is_hovered || is_dragging {
                        lever.thumb_color.lighter(0.1)
                    } else {
                        lever.thumb_color
                    };
                }
            }
        }
    }
}

pub fn update_slider_data(
    sliders: Query<(Entity, &SliderValue, &SliderRange), With<Slider>>,
    children: Query<&Children>,
    mut thumbs: Query<&mut Node, With<SliderThumb>>,
) {
    for (slider_ent, value, range) in sliders.iter() {
        for child in children.iter_descendants(slider_ent) {
            if let Ok(mut node) = thumbs.get_mut(child) {
                node.bottom = percent(range.thumb_position(value.0) * 100.0);
            }
        }
    }
}
