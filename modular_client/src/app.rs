use std::char;

use femtovg::{renderer::OpenGl, Canvas, FontId, Paint};
use glutin::event::{ModifiersState, VirtualKeyCode};

use crate::ui::{
    context::Context,
    edge_insets::EdgeInsets,
    widget::Widget,
    widgets::{
        align::{Align, Alignment},
        padding::Padding,
        text::Text,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Command {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    prompt: String,
    cursor: usize,
    prompt_active: bool,
    modifiers: ModifiersState,
    pub font: FontId,
}

impl App {
    pub fn new(canvas: &mut Canvas<OpenGl>) -> anyhow::Result<Self> {
        let font = canvas.add_font_mem(include_bytes!(
            "assets/Fira_Code_v5.2/ttf/FiraCode-Regular.ttf"
        ))?;
        Ok(Self {
            prompt: String::new(),
            prompt_active: false,
            cursor: 0,
            modifiers: ModifiersState::empty(),
            font,
        })
    }

    pub fn handle_modifier_change(&mut self, modifiers: ModifiersState) {
        self.modifiers = modifiers;
    }

    pub fn handle_char_recieved(&mut self, c: char) {
        match c {
            ':' => {
                self.prompt_active = true;
            }
            c if self.prompt_active && (c.is_ascii_graphic() || c == ' ') => {
                println!("{:?} is ascii {:?}", c, c.is_ascii());
                self.prompt.insert(self.cursor, c);
                self.cursor += 1;
            }
            _ => {}
        }
        println!("char {:?}", self);
    }

    pub fn handle_key_press(&mut self, keycode: VirtualKeyCode) {
        match keycode {
            VirtualKeyCode::Escape if self.prompt_active => {
                self.prompt_active = false;
                self.cursor = 0;
                self.prompt = String::new();
            }
            VirtualKeyCode::Return if self.prompt_active => {
                let mut prompt = String::new();
                std::mem::swap(&mut prompt, &mut self.prompt);
                self.parse_command(prompt);
                self.prompt_active = false;
                self.cursor = 0;
            }
            VirtualKeyCode::Back if self.prompt_active => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.prompt.remove(self.cursor);
                }
            }
            VirtualKeyCode::Left | VirtualKeyCode::Right if self.prompt_active => {
                let mut cursor = self.cursor as i32;
                cursor += if keycode == VirtualKeyCode::Left {
                    -1
                } else {
                    1
                };
                cursor = 0.max((self.prompt.len() as i32).min(cursor));
                self.cursor = cursor as usize;
            }
            _ => {}
        };
        println!("key {:?}", self);
    }

    pub fn parse_command(&mut self, _prompt: String) {}

    pub fn build_prompt(&mut self, context: &Context) -> Padding {
        Padding::new(
            EdgeInsets::all(5.0),
            Align::new(
                Alignment::center_left(),
                Text::new(if self.prompt_active {
                    [":".to_string(), self.prompt.clone()].join("")
                } else {
                    "".to_string()
                })
                .with_fill({
                    let mut paint = Paint::color(context.theme.f_high);
                    paint.set_font_size(30.0);
                    paint.set_font(&[self.font]);
                    paint
                }),
            ),
        )
    }
}
