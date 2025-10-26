use bevy_app::{Plugin, PreUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Changed, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
};
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene2::prelude::*;
use bevy_ui::{px, BorderRadius, Node, PositionType};
use bevy_ui_widgets::{ScrollbarDragState, ScrollbarThumb};

use crate::{cursor::EntityCursor, theme::ThemeBackgroundColor, tokens};

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ScrollbarStyle;

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ScrollbarThumbStyle;

/// Scrollbar scene function.
pub fn scrollbar() -> impl Scene {
    bsn! {
        // Scrollbar {
        //     // target: target,
        //     orientation: orientation,
        //     min_thumb_length: 8.0
        // }
        ScrollbarStyle
        BorderRadius::all(px(3))
        ThemeBackgroundColor(tokens::SCROLLBAR_BG)
        [(
            Node {
                position_type: PositionType::Absolute,
            }
            Hovered
            ThemeBackgroundColor(tokens::SCROLLBAR_THUMB)
            BorderRadius::all(px(3))
            ScrollbarThumb
            ScrollbarThumbStyle
            EntityCursor::System(bevy_window::SystemCursorIcon::Pointer)
        )]
    }
}

fn update_scrollbar_thumb_styles(
    q_thumbs: Query<
        (Entity, &Hovered, &ThemeBackgroundColor, &ScrollbarDragState),
        (
            With<ScrollbarThumbStyle>,
            Or<(Changed<Hovered>, Changed<ScrollbarDragState>)>,
        ),
    >,
    mut commands: Commands,
) {
    for (scrollbar_ent, hovered, bg_color, drag_state) in q_thumbs.iter() {
        let bg_token = if hovered.0 || drag_state.dragging {
            tokens::SCROLLBAR_THUMB_HOVER
        } else {
            tokens::SCROLLBAR_THUMB
        };

        if bg_token != bg_color.0 {
            commands
                .entity(scrollbar_ent)
                .insert(ThemeBackgroundColor(bg_token));
        }
    }
}

/// Plugin which registers the systems for updating the scrollbar styles.
pub struct ScrollbarPlugin;

impl Plugin for ScrollbarPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            update_scrollbar_thumb_styles.in_set(PickingSystems::Last),
        );
    }
}
