use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{Plugin, PreUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::ChildOf,
    lifecycle::{Insert, RemovedComponents},
    observer::On,
    prelude::Name,
    query::{Added, Changed, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs as _,
    system::{Commands, Query},
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_log::info;
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene2::{bsn, Scene, SceneList};
use bevy_ui::{
    px, AlignItems, BorderRadius, Display, FlexDirection, InteractionDisabled, JustifyContent,
    Node, Overflow, PositionType, Selected, UiRect,
};
use bevy_ui_widgets::{
    ActiveDescendant, ControlOrientation, ListBox, ListItem, ScrollArea, Scrollbar,
};

use crate::{
    constants::{fonts, size},
    controls::scrollbar::scrollbar,
    cursor::EntityCursor,
    font_styles::InheritableFont,
    theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeFontColor},
    tokens,
};

/// A container that displays a scrolling list of items
pub fn listview<S: SceneList>(children: S) -> impl Scene {
    bsn! {
        // Outer frame that holds the scrollbar
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            padding: UiRect {
                right: px(10) // Room for scrollbar
            }
        }
        ListBox
        AccessibilityNode(accesskit::Node::new(Role::ListBox))
        TabIndex(0)
        [
            // Inner part that scrolls
            (
                #inner
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    justify_content: JustifyContent::Start,
                    overflow: Overflow::scroll_y(),
                }
                ScrollArea::default()
                [
                    {children}
                ]
            ),
            // Scrollbar
            (
                :scrollbar()
                Node {
                    position_type: PositionType::Absolute,
                    right: px(0),
                    top: px(0),
                    bottom: px(0),
                    width: px(6),
                }
                Scrollbar {
                    orientation: ControlOrientation::Vertical,
                    target: #inner,
                    min_thumb_length: 6.0,
                }
            ),
        ]
    }
}

/// Marker for the listrow check mark
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ListRowStyle;

/// Marker for the listrow check mark
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ActiveRowOutline;

/// A selectable row in a list of items
pub fn listrow() -> impl Scene {
    bsn! {
        Node {
            min_height: size::ROW_HEIGHT,
            min_width: size::ROW_HEIGHT,
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Center,
            padding: UiRect::axes(px(8), px(2)),
        }
        AccessibilityNode(accesskit::Node::new(Role::ListItem))
        ThemeFontColor(tokens::LISTROW_TEXT)
        ThemeBackgroundColor(tokens::LISTROW_BG)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: 14.0,
        }
        Hovered
        ListItem
        ListRowStyle
    }
}

fn update_listrow_styles(
    q_listrows: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Selected>,
            &Hovered,
            &ThemeBackgroundColor,
            &ThemeFontColor,
        ),
        (
            With<ListRowStyle>,
            Or<(
                Changed<Hovered>,
                Added<Selected>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    // q_children: Query<&Children>,
    // mut q_outline: Query<(&ThemeBackgroundColor, &ThemeBorderColor), With<ListRowOutline>>,
    // mut q_mark: Query<&ThemeBorderColor, With<ListRowMark>>,
    mut commands: Commands,
) {
    for (listrow_ent, disabled, selected, hovered, bg_color, font_color) in q_listrows.iter() {
        set_listrow_styles(
            listrow_ent,
            disabled,
            selected,
            hovered.0,
            bg_color,
            font_color,
            &mut commands,
        );
    }
}

fn update_listrow_styles_remove(
    q_listrows: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Selected>,
            &Hovered,
            &ThemeBackgroundColor,
            &ThemeFontColor,
        ),
        With<ListRowStyle>,
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_selected: RemovedComponents<Selected>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_selected.read())
        .for_each(|ent| {
            if let Ok((listrow_ent, disabled, selected, hovered, bg_color, font_color)) =
                q_listrows.get(ent)
            {
                set_listrow_styles(
                    listrow_ent,
                    disabled,
                    selected,
                    hovered.0,
                    bg_color,
                    font_color,
                    &mut commands,
                );
            }
        });
}

fn set_listrow_styles(
    listrow_ent: Entity,
    disabled: bool,
    selected: bool,
    hovered: bool,
    bg_color: &ThemeBackgroundColor,
    font_color: &ThemeFontColor,
    commands: &mut Commands,
) {
    let outline_bg_token = match (disabled, selected, hovered) {
        (false, true, _) => tokens::LISTROW_BG_SELECTED,
        (false, false, true) => tokens::LISTROW_BG_HOVER,
        _ => tokens::LISTROW_BG,
    };

    let font_color_token = match disabled {
        true => tokens::LISTROW_TEXT_DISABLED,
        false => tokens::LISTROW_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
    };

    // Change outline background
    if bg_color.0 != outline_bg_token {
        commands
            .entity(listrow_ent)
            .insert(ThemeBackgroundColor(outline_bg_token));
    }

    // Change font color
    if font_color.0 != font_color_token {
        commands
            .entity(listrow_ent)
            .insert(ThemeFontColor(font_color_token));
    }

    // Change cursor shape
    commands
        .entity(listrow_ent)
        .insert(EntityCursor::System(cursor_shape));
}

fn on_insert_active(
    add: On<Insert, ActiveDescendant>,
    q_active_descendant: Query<&ActiveDescendant>,
    q_row_outline: Query<(Entity, &ChildOf), With<ActiveRowOutline>>,
    mut commands: Commands,
) {
    info!("AD");
    let listbox = add.entity;
    let Ok(active_descendant) = q_active_descendant.get(listbox) else {
        return;
    };

    // Despawn all active outlines that aren't the current active descendant.
    for (outline_id, ChildOf(outline_parent)) in q_row_outline.iter() {
        if !active_descendant.visible || Some(*outline_parent) != active_descendant.item {
            commands.entity(outline_id).despawn();
        }
    }

    if let Some(active_item) = active_descendant.item
        && active_descendant.visible
    {
        commands.entity(active_item).with_child((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                border: UiRect::all(px(2)),
                ..Default::default()
            },
            BorderRadius::all(px(2)),
            ThemeBorderColor(tokens::FOCUS_RING),
            ActiveRowOutline,
        ));
    }
}

/// Plugin which registers the systems for updating the listrow styles.
pub struct ListViewPlugin;

impl Plugin for ListViewPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_listrow_styles, update_listrow_styles_remove).in_set(PickingSystems::Last),
        );
        app.add_observer(on_insert_active);
    }
}
