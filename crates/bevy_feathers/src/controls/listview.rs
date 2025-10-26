use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_ecs::prelude::Name;
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::hover::Hovered;
use bevy_scene2::{bsn, Scene, SceneList};
use bevy_ui::{
    px, AlignItems, Display, FlexDirection, JustifyContent, Node, Overflow, PositionType,
    ScrollPosition, UiRect,
};
use bevy_ui_widgets::{ControlOrientation, Scrollbar};

use crate::{
    constants::{fonts, size},
    controls::scrollbar::scrollbar,
    font_styles::InheritableFont,
    theme::ThemeFontColor,
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
        }
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
                ScrollPosition::default()
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
        ThemeFontColor(tokens::BUTTON_TEXT)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: 14.0,
        }
        Hovered
    }
}
