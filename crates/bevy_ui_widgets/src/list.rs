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
use bevy_ui::{InteractionDisabled, Selectable, Selected};

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
    q_listitems: Query<(Has<Selected>, Has<InteractionDisabled>), With<ListItem>>,
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

            // Find all listbox descendants that are not disabled
            let list_items = q_children
                .iter_descendants(ev.focused_entity)
                .filter_map(|child_id| match q_listitems.get(child_id) {
                    Ok((checked, false)) => Some((child_id, checked)),
                    Ok((_, true)) | Err(_) => None,
                })
                .collect::<Vec<_>>();
            if list_items.is_empty() {
                return; // No enabled rows in the group
            }
            let current_index = list_items
                .iter()
                .position(|(_, checked)| *checked)
                .unwrap_or(usize::MAX); // Default to invalid index if none are checked

            let next_index = match key_code {
                KeyCode::ArrowUp | KeyCode::ArrowLeft => {
                    // Navigate to the previous list row in the group
                    if current_index == 0 || current_index >= list_items.len() {
                        // If we're at the first one, wrap around to the last
                        list_items.len() - 1
                    } else {
                        // Move to the previous one
                        current_index - 1
                    }
                }
                KeyCode::ArrowDown | KeyCode::ArrowRight => {
                    // Navigate to the next list row in the group
                    if current_index >= list_items.len() - 1 {
                        // If we're at the last one, wrap around to the first
                        0
                    } else {
                        // Move to the next one
                        current_index + 1
                    }
                }
                KeyCode::Home => {
                    // Navigate to the first list row in the group
                    0
                }
                KeyCode::End => {
                    // Navigate to the last list row in the group
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

            // Trigger the on_change event for the newly selected row
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
    q_listitems: Query<(Has<Selected>, Has<InteractionDisabled>), With<ListItem>>,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    if q_listbox.contains(ev.entity) {
        // Starting with the original target, search upward for a list row.
        let row_id = if q_listitems.contains(ev.original_event_target()) {
            ev.original_event_target()
        } else {
            // Search ancestors for the first list row
            let mut found_row = None;
            for ancestor in q_parents.iter_ancestors(ev.original_event_target()) {
                if q_listbox.contains(ancestor) {
                    // We reached a list box before finding a list row, bail out
                    return;
                }
                if q_listitems.contains(ancestor) {
                    found_row = Some(ancestor);
                    break;
                }
            }

            match found_row {
                Some(row) => row,
                None => return, // No list row found in the ancestor chain
            }
        };

        // List row is disabled.
        if q_listitems.get(row_id).unwrap().1 {
            return;
        }

        // Gather all the enabled list box descendants for exclusion.
        let all_rows = q_children
            .iter_descendants(ev.entity)
            .filter_map(|child_id| match q_listitems.get(child_id) {
                Ok((selected, false)) => Some((child_id, selected)),
                Ok((_, true)) | Err(_) => None,
            })
            .collect::<Vec<_>>();

        if all_rows.is_empty() {
            return; // No enabled list rows in the group
        }

        // Pick out the list row that is currently checked.
        ev.propagate(false);
        let current_row = all_rows
            .iter()
            .find(|(_, checked)| *checked)
            .map(|(id, _)| *id);

        if current_row == Some(row_id) {
            // If they clicked the currently checked list row, do nothing
            return;
        }

        // Trigger the on_change event for the newly checked list row
        commands.trigger(ValueChange::<Entity> {
            source: ev.entity,
            value: row_id,
        });
    }
}

/// Plugin that adds the observers for the [`ListBox`] widget.
pub struct ListBoxPlugin;

impl Plugin for ListBoxPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(listbox_on_key_input)
            .add_observer(listbox_on_row_click);
    }
}

/// Observer function for updating list row selection state.
pub fn listbox_update_selection(
    value_change: On<ValueChange<Entity>>,
    q_listbox: Query<(), With<ListBox>>,
    q_listitems: Query<(Has<Selected>, Has<InteractionDisabled>), With<ListItem>>,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    {
        let change = value_change.event();
        let row = change.value;

        // Find the ListBox that this change applies to. Prefer the event source if it's a ListBox,
        // otherwise walk the ancestors of the row to find the containing ListBox.
        let listbox = if q_listbox.contains(change.source) {
            change.source
        } else {
            // requires: q_parents: Query<&ChildOf>
            let mut found = None;
            for ancestor in q_parents.iter_ancestors(row) {
                if q_listbox.contains(ancestor) {
                    found = Some(ancestor);
                    break;
                }
            }
            match found {
                Some(lb) => lb,
                None => return, // no containing ListBox found
            }
        };

        // Gather all enabled list items that are descendants of the found ListBox.
        // requires: q_children: Query<&Children>
        let enabled_rows = q_children
            .iter_descendants(listbox)
            .filter_map(|child_id| match q_listitems.get(child_id) {
                Ok((has_selected, false)) => Some((child_id, has_selected)),
                _ => None,
            })
            .collect::<Vec<_>>();

        if enabled_rows.is_empty() {
            return;
        }

        // If the changed row isn't one of the enabled rows in this listbox, ignore.
        if !enabled_rows.iter().any(|(id, _)| *id == row) {
            return;
        }

        // Determine currently selected row (if any).
        let current_selected = enabled_rows
            .iter()
            .find(|(_, checked)| *checked)
            .map(|(id, _)| *id);

        // If the selection hasn't changed, do nothing.
        if current_selected == Some(row) {
            return;
        }

        // Update Selected component: insert for the new row, remove for others.
        for (id, _) in enabled_rows {
            if id == row {
                commands.entity(id).insert(Selected);
            } else {
                commands.entity(id).remove::<Selected>();
            }
        }
    }
}
