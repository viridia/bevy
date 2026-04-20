use bevy_app::PropagateOver;
use bevy_asset::AssetServer;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    lifecycle::Insert,
    observer::On,
    query::With,
    relationship::Relationship,
    system::{Commands, Query, Res},
    template::template,
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input_focus::{FocusLost, FocusedInput, InputFocus};
use bevy_log::warn;
use bevy_scene::prelude::*;
use bevy_text::{
    EditableText, EditableTextFilter, FontSource, FontWeight, TextEdit, TextEditChange, TextFont,
};
use bevy_ui::{px, widget::Text, AlignItems, AlignSelf, Display, JustifyContent, Node, UiRect};
use bevy_ui_widgets::ValueChange;

use crate::{
    constants::{fonts, size},
    controls::{text_input, text_input_container, TextInputProps},
    theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemeToken},
    tokens,
};

/// Marker to indicate a number input widget with feathers styling.
#[derive(Component, Default, Clone)]
struct FeathersNumberInput;

/// Used to indicate what format of numbers we are editing.
///
/// Note that this does not affect the type of value change event (which is still
/// [`ValueChange<f64>`]) or the type of [`NumberInputValue(f64)`]. It only controls the internal
/// parsing and number formatting precision.
#[derive(Component, Default, Clone, Copy)]
pub enum NumberFormat {
    /// A 32-bit float
    #[default]
    F32,
    /// A 64-bit float
    F64,
    /// A 32-bit integer
    I32,
}

/// Parameters for the text input template, passed to [`number_input`] function.
pub struct NumberInputProps {
    /// The "sigil" is a colored strip along the left edge of the input, which is used to
    /// distinguish between different axes. The default is transparent (no sigil).
    pub sigil_color: ThemeToken,
    /// A caption to be placed on the left side of the input, next to the colored stripe.
    /// Usually one of "X", "Y" or "Z".
    pub label_text: Option<&'static str>,
    /// Indicate what size numbers we are editing.
    pub number_format: NumberFormat,
}

impl Default for NumberInputProps {
    fn default() -> Self {
        Self {
            sigil_color: tokens::TEXT_INPUT_BG,
            label_text: None,
            number_format: NumberFormat::F32,
        }
    }
}

/// A component which, when inserted, will update the displayed value of the number being edited.
/// This avoids the need for users to search the internal hierarchy of the widget looking for the
/// text buffer (which is not located at the root level).
// TODO: Consider unifying this with bevy_ui_widgets::SliderValue
#[derive(Component, Debug, Default, PartialEq, Clone, Copy)]
#[component(immutable)]
pub struct NumberInputValue(pub f64);

/// Widget that permits text entry of floating-point numbers. This widget implements two-way
/// synchronization: when the widget has focus, it emits values (via a [`ValueChange<f64>`]) event
/// as the user types; when the widget does not have focus, it listens for changes to the
/// [`NumberInputValue`] component. To avoid excessive updating, you should only replace the
/// input value component when there is an actual change, that is, when the new value is different
/// from the current value. This will cause the text to be replaced.
///
/// In most cases, the actual source of truth for the numeric value will be external, that is,
/// some property in an app-specific data structure. It's the responsibility of the app to
/// sychronize this value with the [`number_input`] widget in both directions:
/// * When a [`ValueChange`] event is received, update the app-specific property.
/// * When the app-specific property changes - either in response to a [`ValueChange`] event, or
///   because of some other action, insert a [`NumberInputValue`] component to update the
///   displayed value.
// TODO: Add field validation when it becomes available.
pub fn number_input(props: NumberInputProps) -> impl Scene {
    bsn! {
        :text_input_container()
        ThemeBorderColor({props.sigil_color.clone()})
        FeathersNumberInput
        template_value(props.number_format)
        NumberInputValue(0.0)
        on(number_input_on_value_change)
        Children [
            {
                match props.label_text {
                    Some(text) => Box::new(bsn_list!(
                        Node {
                            display: Display::Flex,
                            align_items: AlignItems::Center,
                            align_self: AlignSelf::Stretch,
                            justify_content: JustifyContent::Center,
                            padding: UiRect::axes(px(6), px(0)),
                        }
                        ThemeBackgroundColor(tokens::TEXT_INPUT_LABEL_BG)
                        Children [
                            Text::new(text.to_string())
                            template(|ctx| {
                                Ok(TextFont {
                                    font: FontSource::Handle(ctx.resource::<AssetServer>().load(fonts::REGULAR)),
                                    font_size: size::COMPACT_FONT,
                                    weight: FontWeight::NORMAL,
                                    ..Default::default()
                                })
                            })
                            PropagateOver<TextFont>
                            ThemeTextColor(tokens::TEXT_INPUT_TEXT)
                        ]
                    )) as Box<dyn SceneList>,
                    None => Box::new(bsn_list!()) as Box<dyn SceneList>
                }
            }
            text_input(TextInputProps {
                visible_width: None,
                max_characters: Some(20),
            })
            on(number_input_on_text_change)
            on(number_input_on_enter_key)
            on(number_input_on_focus_loss)
            EditableTextFilter::new(|c| {
                c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')
            }),
        ]
    }
}

fn number_input_on_text_change(
    change: On<TextEditChange>,
    q_parent: Query<&ChildOf>,
    q_number_input: Query<&NumberFormat, With<FeathersNumberInput>>,
    q_text_input: Query<&EditableText>,
    mut commands: Commands,
) {
    let Ok(parent) = q_parent.get(change.event_target()) else {
        return;
    };

    let Ok(number_format) = q_number_input.get(parent.get()) else {
        return;
    };

    let Ok(editable_text) = q_text_input.get(change.event_target()) else {
        return;
    };

    let text_value = editable_text.value().to_string();
    emit_value_change(text_value, *number_format, parent.0, &mut commands);
}

fn number_input_on_value_change(
    change: On<Insert, NumberInputValue>,
    q_children: Query<&Children>,
    q_number_input: Query<(&NumberFormat, &NumberInputValue), With<FeathersNumberInput>>,
    mut q_text_input: Query<&mut EditableText>,
    focus: Res<InputFocus>,
) {
    let Ok((number_format, number_input_value)) = q_number_input.get(change.event_target()) else {
        return;
    };

    let Ok(children) = q_children.get(change.event_target()) else {
        return;
    };

    for child_id in children.iter() {
        if focus.get() != Some(*child_id)
            && let Ok(mut editable_text) = q_text_input.get_mut(*child_id)
        {
            let new_digits = format_value(number_input_value.0, *number_format);
            editable_text.queue_edit(TextEdit::SelectAll);
            editable_text.queue_edit(TextEdit::Insert(new_digits.into()));
            break;
        }
    }
}

fn number_input_on_enter_key(
    key_input: On<FocusedInput<KeyboardInput>>,
    q_parent: Query<&ChildOf>,
    q_number_input: Query<&NumberFormat, With<FeathersNumberInput>>,
    q_text_input: Query<&EditableText>,
    mut commands: Commands,
) {
    if key_input.input.key_code != KeyCode::Enter {
        return;
    }

    let Ok(parent) = q_parent.get(key_input.event_target()) else {
        return;
    };

    let Ok(number_format) = q_number_input.get(parent.get()) else {
        return;
    };

    let Ok(editable_text) = q_text_input.get(key_input.event_target()) else {
        return;
    };

    let text_value = editable_text.value().to_string();
    emit_value_change(text_value, *number_format, parent.0, &mut commands);
}

fn number_input_on_focus_loss(
    focus_lost: On<FocusLost>,
    q_parent: Query<&ChildOf>,
    q_number_input: Query<&NumberFormat, With<FeathersNumberInput>>,
    mut q_text_input: Query<&mut EditableText>,
    mut commands: Commands,
) {
    let editable_text_id = focus_lost.event_target();

    let Ok(parent) = q_parent.get(editable_text_id) else {
        return;
    };

    let Ok(number_format) = q_number_input.get(parent.get()) else {
        return;
    };

    let Ok(editable_text) = q_text_input.get_mut(editable_text_id) else {
        return;
    };

    let text_value = editable_text.value().to_string();
    emit_value_change(text_value, *number_format, parent.0, &mut commands);
}

fn emit_value_change(
    text_value: String,
    format: NumberFormat,
    source: Entity,
    commands: &mut Commands,
) {
    let number_value = match format {
        NumberFormat::F32 => text_value.parse::<f32>().map(|f| f as f64).map_err(|_| ()),
        NumberFormat::F64 => text_value.parse::<f64>().map_err(|_| ()),
        NumberFormat::I32 => text_value.parse::<i32>().map(|f| f as f64).map_err(|_| ()),
    };

    match number_value {
        Ok(new_value) => {
            commands.trigger(ValueChange {
                source,
                value: new_value,
                is_final: true,
            });
        }
        Err(_) => {
            // TODO: Emit a validation error once these are defined
            warn!("Invalid floating-point number in text edit");
        }
    }
}

fn format_value(value: f64, format: NumberFormat) -> String {
    match format {
        NumberFormat::F32 => format!("{}", value as f32),
        NumberFormat::F64 => format!("{}", value),
        NumberFormat::I32 => format!("{}", value as i32),
    }
}

/// Observer function which updates the [`NumberInputValue`] in response to a [`ValueChange`] event.
pub fn number_input_self_update(
    value_change: On<ValueChange<f64>>,
    mut q_number_input: Query<&NumberInputValue>,
    mut commands: Commands,
) {
    let Ok(number_input) = q_number_input.get_mut(value_change.event_target()) else {
        return;
    };

    if number_input.0 != value_change.value {
        commands
            .entity(value_change.source)
            .insert(NumberInputValue(value_change.value));
    }
}
