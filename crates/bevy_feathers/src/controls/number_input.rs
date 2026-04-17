use bevy_app::PropagateOver;
use bevy_asset::AssetServer;
use bevy_ecs::{
    component::Component,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    lifecycle::Insert,
    observer::On,
    query::With,
    relationship::Relationship,
    system::{Commands, Query, Res},
    template::template,
};
use bevy_input_focus::{FocusLost, InputFocus};
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

/// Parameters for the text input template, passed to [`number_input`] function.
pub struct NumberInputProps {
    /// The "sigil" is a colored strip along the left edge of the input, which is used to
    /// distinguish between different axes. The default is transparent (no sigil).
    pub sigil_color: ThemeToken,
    /// A caption to be placed on the left side of the input, next to the colored stripe.
    /// Usually one of "X", "Y" or "Z".
    pub label_text: Option<&'static str>,
}

impl Default for NumberInputProps {
    fn default() -> Self {
        Self {
            sigil_color: tokens::TEXT_INPUT_BG,
            label_text: None,
        }
    }
}

/// A component which stores the current value of the number input. Inserting this will
/// update the displayed value of the widget.
// TODO: Consider unifying this with bevy_ui_widgets::SliderValue
#[derive(Component, Debug, Default, PartialEq, Clone, Copy)]
#[component(immutable)]
pub struct NumberInputValue(pub f32);

/// Widget that permits text entry of floating-point numbers. This widget implements two-way
/// synchronization: when the widget has focus, it emits values (via a [`ValueChange<f32>`]) event
/// as the user types; when the widget does not have focus, it listens for changes to the
/// [`NumberInputValue`] component. To avoid excessive updating, you should only replace the
/// input value component when there is an actual change, that is, when the new value is different
/// from the current value. This will cause the text to be replaced.
// TODO: Add field validation when it becomes available.
pub fn number_input(props: NumberInputProps) -> impl Scene {
    bsn! {
        :text_input_container()
        ThemeBorderColor({props.sigil_color.clone()})
        FeathersNumberInput
        NumberInputValue(0.0)
        on(on_number_value_change)
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
            // {props.sigil_text.map(|text| Box::new(bsn_list!(Text::new(text.to_string()))) as Box<dyn SceneList>)},
            text_input(TextInputProps {
                visible_width: None,
                max_characters: Some(20),
            })
            on(on_number_text_change)
            on(on_focus_loss)
            EditableTextFilter::new(|c| {
                c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')
            }),
        ]
    }
}

fn on_number_text_change(
    change: On<TextEditChange>,
    q_parent: Query<&ChildOf>,
    q_number_input: Query<(), With<FeathersNumberInput>>,
    q_text_input: Query<&EditableText>,
    mut commands: Commands,
) {
    let Ok(parent) = q_parent.get(change.event_target()) else {
        return;
    };

    if !q_number_input.contains(parent.get()) {
        return;
    }

    let Ok(editable_text) = q_text_input.get(change.event_target()) else {
        return;
    };

    let text_value = editable_text.value().to_string();
    match text_value.parse::<f32>() {
        Ok(new_value) => {
            commands.trigger(ValueChange {
                source: parent.0,
                value: new_value,
            });
        }
        Err(_) => {
            // TODO: Emit a validation error once these are defined
            warn!("Invalid floating-point number in text edit");
        }
    }
}

fn on_number_value_change(
    change: On<Insert, NumberInputValue>,
    q_children: Query<&Children>,
    q_number_input: Query<&NumberInputValue, With<FeathersNumberInput>>,
    mut q_text_input: Query<&mut EditableText>,
    focus: Res<InputFocus>,
) {
    let Ok(number_input_value) = q_number_input.get(change.event_target()) else {
        return;
    };

    let Ok(children) = q_children.get(change.event_target()) else {
        return;
    };

    for child_id in children.iter() {
        if focus.get() != Some(*child_id)
            && let Ok(mut editable_text) = q_text_input.get_mut(*child_id)
        {
            editable_text.queue_edit(TextEdit::SelectAll);
            editable_text.queue_edit(TextEdit::Insert(format!("{}", number_input_value.0).into()));
            break;
        }
    }
}

/// When we lose focus, apply any changes in [`NumberInputValue`] to the text buffer, in case
/// that there was a programmatic value change while we were editing. Ideally, this will always
/// be true since the app response to a [`ValueChange`] should be to update the value.
fn on_focus_loss(
    focus_lost: On<FocusLost>,
    q_parent: Query<&ChildOf>,
    q_number_input: Query<&NumberInputValue, With<FeathersNumberInput>>,
    mut q_text_input: Query<&mut EditableText>,
    focus: Res<InputFocus>,
) {
    let editable_text_id = focus_lost.event_target();

    let Ok(parent) = q_parent.get(editable_text_id) else {
        return;
    };

    let Ok(number_input_value) = q_number_input.get(parent.get()) else {
        return;
    };

    let Ok(mut editable_text) = q_text_input.get_mut(editable_text_id) else {
        return;
    };

    if focus.get() != Some(editable_text_id) {
        let new_text = format!("{}", number_input_value.0);
        let current_text: String = editable_text.value().to_string();
        if new_text != current_text {
            editable_text.queue_edit(TextEdit::SelectAll);
            editable_text.queue_edit(TextEdit::Insert(new_text.into()));
        }
    }
}

/// Observer function which updates the [`NumberInputValue`] in response to a [`ValueChange`] event.
pub fn number_input_self_update(value_change: On<ValueChange<f32>>, mut commands: Commands) {
    commands
        .entity(value_change.source)
        .insert(NumberInputValue(value_change.value));
}
