use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{Has, With},
    reflect::ReflectComponent,
    system::{Commands, Query},
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input::ButtonState;
use bevy_input_focus::FocusedInput;
use bevy_picking::events::{Click, Pointer};
use bevy_reflect::Reflect;
use bevy_ui::{Checked, InteractionDisabled, Selectable};

use crate::ValueChange;

/// Headless widget implementation for a list box. This component contains multiple [`ListItem`]
/// entities. It implements the tab navigation logic and keyboard shortcuts for list items.
#[derive(Component, Debug, Clone, Default)]
#[require(AccessibilityNode(accesskit::Node::new(Role::ListBox)))]
pub struct ListBox;

/// Marker component that indicates we want to support multiple selection of list items.
#[derive(Component, Debug, Clone, Default)]
pub struct ListBoxMultiSelect;

/// Headless widget implementation for listbox items. These should be enclosed within a
/// [`ListBox`] widget, which is responsible for the mutual exclusion logic.
#[derive(Component, Debug, Clone, Default)]
#[require(AccessibilityNode(accesskit::Node::new(Role::ListItem)), Selectable)]
#[derive(Reflect)]
#[reflect(Component)]
pub struct ListItem;

fn listbox_on_key_input(
    mut ev: On<FocusedInput<KeyboardInput>>,
    q_listbox: Query<(), With<ListBox>>,
    q_listitems: Query<(Has<Checked>, Has<InteractionDisabled>), With<ListItem>>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    if q_listbox.contains(ev.focused_entity) {
        let event = &ev.event().input;
        if event.state == ButtonState::Pressed
            && !event.repeat
            && matches!(
                event.key_code,
                KeyCode::ArrowUp
                    | KeyCode::ArrowDown
                    | KeyCode::ArrowLeft
                    | KeyCode::ArrowRight
                    | KeyCode::Home
                    | KeyCode::End
            )
        {
            let key_code = event.key_code;
            ev.propagate(false);

            // Find all radio descendants that are not disabled
            let list_items = q_children
                .iter_descendants(ev.focused_entity)
                .filter_map(|child_id| match q_listitems.get(child_id) {
                    Ok((checked, false)) => Some((child_id, checked)),
                    Ok((_, true)) | Err(_) => None,
                })
                .collect::<Vec<_>>();
            if list_items.is_empty() {
                return; // No enabled radio buttons in the group
            }
            let current_index = list_items
                .iter()
                .position(|(_, checked)| *checked)
                .unwrap_or(usize::MAX); // Default to invalid index if none are checked

            let next_index = match key_code {
                KeyCode::ArrowUp | KeyCode::ArrowLeft => {
                    // Navigate to the previous radio button in the group
                    if current_index == 0 || current_index >= list_items.len() {
                        // If we're at the first one, wrap around to the last
                        list_items.len() - 1
                    } else {
                        // Move to the previous one
                        current_index - 1
                    }
                }
                KeyCode::ArrowDown | KeyCode::ArrowRight => {
                    // Navigate to the next radio button in the group
                    if current_index >= list_items.len() - 1 {
                        // If we're at the last one, wrap around to the first
                        0
                    } else {
                        // Move to the next one
                        current_index + 1
                    }
                }
                KeyCode::Home => {
                    // Navigate to the first radio button in the group
                    0
                }
                KeyCode::End => {
                    // Navigate to the last radio button in the group
                    list_items.len() - 1
                }
                _ => {
                    return;
                }
            };

            if current_index == next_index {
                // If the next index is the same as the current, do nothing
                return;
            }

            let (next_id, _) = list_items[next_index];

            // Trigger the on_change event for the newly checked radio button
            commands.trigger(ValueChange::<Entity> {
                source: ev.focused_entity,
                value: next_id,
            });
        }
    }
}

fn listbox_on_row_click(
    mut ev: On<Pointer<Click>>,
    q_listbox: Query<(), With<ListBox>>,
    q_listitems: Query<(Has<Checked>, Has<InteractionDisabled>), With<ListItem>>,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    if q_listbox.contains(ev.entity) {
        // Starting with the original target, search upward for a radio button.
        let radio_id = if q_listitems.contains(ev.original_event_target()) {
            ev.original_event_target()
        } else {
            // Search ancestors for the first radio button
            let mut found_radio = None;
            for ancestor in q_parents.iter_ancestors(ev.original_event_target()) {
                if q_listbox.contains(ancestor) {
                    // We reached a radio group before finding a radio button, bail out
                    return;
                }
                if q_listitems.contains(ancestor) {
                    found_radio = Some(ancestor);
                    break;
                }
            }

            match found_radio {
                Some(radio) => radio,
                None => return, // No radio button found in the ancestor chain
            }
        };

        // Radio button is disabled.
        if q_listitems.get(radio_id).unwrap().1 {
            return;
        }

        // Gather all the enabled radio group descendants for exclusion.
        let radio_buttons = q_children
            .iter_descendants(ev.entity)
            .filter_map(|child_id| match q_listitems.get(child_id) {
                Ok((checked, false)) => Some((child_id, checked)),
                Ok((_, true)) | Err(_) => None,
            })
            .collect::<Vec<_>>();

        if radio_buttons.is_empty() {
            return; // No enabled radio buttons in the group
        }

        // Pick out the radio button that is currently checked.
        ev.propagate(false);
        let current_radio = radio_buttons
            .iter()
            .find(|(_, checked)| *checked)
            .map(|(id, _)| *id);

        if current_radio == Some(radio_id) {
            // If they clicked the currently checked radio button, do nothing
            return;
        }

        // Trigger the on_change event for the newly checked radio button
        commands.trigger(ValueChange::<Entity> {
            source: ev.entity,
            value: radio_id,
        });
    }
}

/// Plugin that adds the observers for the [`RadioGroup`] widget.
pub struct ListBoxGroupPlugin;

impl Plugin for ListBoxGroupPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(listbox_on_key_input)
            .add_observer(listbox_on_row_click);
    }
}
