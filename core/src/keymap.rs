use std::sync::Arc;

use druid::{
    kurbo::Line,
    piet::{
        PietTextLayout, Svg, Text, TextAttribute, TextLayout, TextLayoutBuilder,
    },
    text::Attribute,
    BoxConstraints, Command, Data, Env, Event, EventCtx, FontFamily, FontWeight,
    LayoutCtx, LifeCycle, LifeCycleCtx, MouseEvent, PaintCtx, Point, Rect,
    RenderContext, Size, Target, UpdateCtx, Widget, WidgetExt, WidgetId, WidgetPod,
};

use crate::{
    command::{LapceCommandNew, LapceUICommand, LAPCE_UI_COMMAND},
    config::LapceTheme,
    data::LapceTabData,
    editor::LapceEditorView,
    keypress::{
        paint_key, Alignment, DefaultKeyPressHandler, KeyMap, KeyPress, KeyPressData,
    },
    scroll::LapceScrollNew,
    split::{keybinding_to_string, LapceSplitNew},
    state::Mode,
};

pub struct LapceKeymap {
    widget_id: WidgetId,
    active_keymap: Option<(KeyMap, Vec<KeyPress>)>,
    keymap_confirm: Rect,
    keymap_cancel: Rect,
    line_height: f64,
}

impl LapceKeymap {
    pub fn new(data: &LapceTabData) -> Box<dyn Widget<LapceTabData>> {
        let keymap = Self {
            widget_id: data.settings.keymap_widget_id,
            active_keymap: None,
            line_height: 35.0,
            keymap_confirm: Rect::ZERO,
            keymap_cancel: Rect::ZERO,
        };
        let keymap = LapceScrollNew::new(keymap);

        let input = LapceEditorView::new(data.settings.keymap_view_id)
            .hide_header()
            .hide_gutter()
            .padding((15.0, 15.0));
        let header = LapceKeymapHeader::new();
        let split = LapceSplitNew::new(data.settings.keymap_split_id)
            .horizontal()
            .with_child(input.boxed(), None, 55.0)
            .with_child(header.boxed(), None, 55.0)
            .with_flex_child(keymap.boxed(), None, 1.0);

        split.boxed()
    }

    fn mouse_down(&mut self, ctx: &mut EventCtx, pos: Point, data: &LapceTabData) {
        if let Some((keymap, keys)) = self.active_keymap.as_ref() {
            if self.keymap_confirm.contains(pos) {
                ctx.submit_command(Command::new(
                    LAPCE_UI_COMMAND,
                    LapceUICommand::UpdateKeymap(keymap.clone(), keys.clone()),
                    Target::Widget(data.id),
                ));
                self.active_keymap = None;
                return;
            }
            if self.keymap_cancel.contains(pos) {
                self.active_keymap = None;
                return;
            }
            return;
        }
        let commands_with_keymap = if data.keypress.filter_pattern == "" {
            &data.keypress.commands_with_keymap
        } else {
            &data.keypress.filtered_commands_with_keymap
        };

        let commands_without_keymap = if data.keypress.filter_pattern == "" {
            &data.keypress.commands_without_keymap
        } else {
            &data.keypress.filtered_commands_without_keymap
        };

        let i = (pos.y / self.line_height).floor() as usize;
        if i < commands_with_keymap.len() {
            let keymap = commands_with_keymap[i].clone();
            self.active_keymap = Some((keymap, Vec::new()));
        } else {
            let j = i - commands_with_keymap.len();
            if let Some(command) = commands_without_keymap.get(j) {
                self.active_keymap = Some((
                    KeyMap {
                        command: command.cmd.clone(),
                        key: Vec::new(),
                        modes: Vec::new(),
                        when: None,
                    },
                    Vec::new(),
                ));
            }
        }
    }

    fn request_focus(&self, ctx: &mut EventCtx, data: &mut LapceTabData) {
        data.focus = self.widget_id;
        ctx.request_focus();
    }
}

impl Widget<LapceTabData> for LapceKeymap {
    fn id(&self) -> Option<WidgetId> {
        Some(self.widget_id)
    }

    fn event(
        &mut self,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut LapceTabData,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(LAPCE_UI_COMMAND) => {
                let command = cmd.get_unchecked(LAPCE_UI_COMMAND);
                match command {
                    LapceUICommand::Focus => {
                        self.request_focus(ctx, data);
                    }
                    _ => (),
                }
            }
            Event::MouseMove(mouse_event) => {
                ctx.set_handled();
            }
            Event::MouseDown(mouse_event) => {
                ctx.set_handled();
                self.request_focus(ctx, data);
                self.mouse_down(ctx, mouse_event.pos, data);
                ctx.request_paint();
            }
            Event::KeyDown(key_event) => {
                if let Some((keymap, keys)) = self.active_keymap.as_mut() {
                    if let Some(keypress) = KeyPressData::keypress(key_event) {
                        if keys.len() == 2 {
                            keys.clear();
                        }
                        keys.push(keypress);
                        ctx.request_paint();
                    }
                } else {
                    let mut keypress = data.keypress.clone();
                    Arc::make_mut(&mut keypress).key_down(
                        ctx,
                        key_event,
                        &mut DefaultKeyPressHandler {},
                        env,
                    );
                }
            }
            _ => (),
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &LapceTabData,
        env: &Env,
    ) {
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &LapceTabData,
        data: &LapceTabData,
        env: &Env,
    ) {
        if !data
            .keypress
            .commands_with_keymap
            .same(&old_data.keypress.commands_with_keymap)
            || !data
                .keypress
                .commands_without_keymap
                .same(&old_data.keypress.commands_without_keymap)
            || data.keypress.filter_pattern != old_data.keypress.filter_pattern
            || !data
                .keypress
                .filtered_commands_with_keymap
                .same(&old_data.keypress.filtered_commands_with_keymap)
            || !data
                .keypress
                .filtered_commands_without_keymap
                .same(&old_data.keypress.filtered_commands_without_keymap)
        {
            ctx.request_layout();
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &LapceTabData,
        env: &Env,
    ) -> Size {
        let commands_with_keymap = if data.keypress.filter_pattern == "" {
            &data.keypress.commands_with_keymap
        } else {
            &data.keypress.filtered_commands_with_keymap
        };

        let commands_without_keymap = if data.keypress.filter_pattern == "" {
            &data.keypress.commands_without_keymap
        } else {
            &data.keypress.filtered_commands_without_keymap
        };

        Size::new(
            bc.max().width,
            (self.line_height
                * (commands_with_keymap.len() + commands_without_keymap.len())
                    as f64)
                .max(bc.max().height),
        )
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &LapceTabData, env: &Env) {
        let size = ctx.size();
        let rect = ctx.region().bounding_box();
        let start = (rect.y0 / self.line_height).floor() as usize;
        let end = (rect.y1 / self.line_height).ceil() as usize;
        let keypress_width = 200.0;

        let commands_with_keymap = if data.keypress.filter_pattern == "" {
            &data.keypress.commands_with_keymap
        } else {
            &data.keypress.filtered_commands_with_keymap
        };

        let commands_without_keymap = if data.keypress.filter_pattern == "" {
            &data.keypress.commands_without_keymap
        } else {
            &data.keypress.filtered_commands_without_keymap
        };

        let commands_with_keymap_len = commands_with_keymap.len();
        for i in start..end + 1 {
            if i % 2 == 0 {
                ctx.fill(
                    Size::new(rect.width(), self.line_height)
                        .to_rect()
                        .with_origin(Point::new(
                            rect.x0,
                            self.line_height * i as f64,
                        )),
                    data.config
                        .get_color_unchecked(LapceTheme::EDITOR_CURRENT_LINE),
                );
            }
            if i < commands_with_keymap_len {
                let keymap = &commands_with_keymap[i];
                if let Some(cmd) = data.keypress.commands.get(&keymap.command) {
                    let text_layout = ctx
                        .text()
                        .new_text_layout(
                            cmd.palette_desc.clone().unwrap_or(cmd.cmd.clone()),
                        )
                        .font(FontFamily::SYSTEM_UI, 13.0)
                        .text_color(
                            data.config
                                .get_color_unchecked(LapceTheme::EDITOR_FOREGROUND)
                                .clone(),
                        )
                        .build()
                        .unwrap();
                    let text_size = text_layout.size();
                    ctx.draw_text(
                        &text_layout,
                        Point::new(
                            10.0,
                            i as f64 * self.line_height
                                + (self.line_height - text_size.height) / 2.0,
                        ),
                    );
                }

                let origin = Point::new(
                    size.width / 2.0 - keypress_width + 10.0,
                    i as f64 * self.line_height + self.line_height / 2.0,
                );
                keymap.paint(ctx, origin, Alignment::Left, &data.config);

                if let Some(condition) = keymap.when.as_ref() {
                    let text_layout = ctx
                        .text()
                        .new_text_layout(condition.to_string())
                        .font(FontFamily::SYSTEM_UI, 13.0)
                        .text_color(
                            data.config
                                .get_color_unchecked(LapceTheme::EDITOR_FOREGROUND)
                                .clone(),
                        )
                        .build()
                        .unwrap();
                    let text_size = text_layout.size();
                    ctx.draw_text(
                        &text_layout,
                        Point::new(
                            size.width / 2.0
                                + 10.0
                                + if data.config.lapce.modal {
                                    keypress_width
                                } else {
                                    0.0
                                },
                            i as f64 * self.line_height
                                + (self.line_height - text_size.height) / 2.0,
                        ),
                    )
                }

                if data.config.lapce.modal {
                    if keymap.modes.len() > 0 {
                        let mut origin = Point::new(
                            size.width / 2.0 + 10.0,
                            i as f64 * self.line_height + self.line_height / 2.0,
                        );
                        for mode in keymap.modes.iter() {
                            let mode = match mode {
                                Mode::Normal => "Normal",
                                Mode::Insert => "Insert",
                                Mode::Visual => "Visual",
                                Mode::Terminal => "Terminal",
                            };
                            let (rect, text_layout, text_layout_pos) =
                                paint_key(ctx, mode, origin, &data.config);
                            ctx.draw_text(&text_layout, text_layout_pos);
                            ctx.stroke(
                                rect,
                                data.config
                                    .get_color_unchecked(LapceTheme::LAPCE_BORDER),
                                1.0,
                            );
                            origin += (rect.width() + 5.0, 0.0);
                        }
                    }
                }
            } else {
                let j = i - commands_with_keymap_len;
                if let Some(command) = commands_without_keymap.get(j) {
                    let text_layout = ctx
                        .text()
                        .new_text_layout(
                            command
                                .palette_desc
                                .clone()
                                .unwrap_or(command.cmd.clone()),
                        )
                        .font(FontFamily::SYSTEM_UI, 13.0)
                        .text_color(
                            data.config
                                .get_color_unchecked(LapceTheme::EDITOR_FOREGROUND)
                                .clone(),
                        )
                        .build()
                        .unwrap();
                    let text_size = text_layout.size();
                    ctx.draw_text(
                        &text_layout,
                        Point::new(
                            10.0,
                            i as f64 * self.line_height
                                + (self.line_height - text_size.height) / 2.0,
                        ),
                    )
                }
            }
        }

        let x = size.width / 2.0 - keypress_width;
        ctx.stroke(
            Line::new(Point::new(x, 0.0), Point::new(x, size.height)),
            data.config.get_color_unchecked(LapceTheme::LAPCE_BORDER),
            1.0,
        );
        let x = size.width / 2.0;
        ctx.stroke(
            Line::new(Point::new(x, 0.0), Point::new(x, size.height)),
            data.config.get_color_unchecked(LapceTheme::LAPCE_BORDER),
            1.0,
        );
        if data.config.lapce.modal {
            let x = size.width / 2.0 + keypress_width;
            ctx.stroke(
                Line::new(Point::new(x, 0.0), Point::new(x, size.height)),
                data.config.get_color_unchecked(LapceTheme::LAPCE_BORDER),
                1.0,
            );
        }

        if let Some((keymap, keys)) = self.active_keymap.as_ref() {
            let paint_rect = rect.clone();
            let size = paint_rect.size();
            let active_width = 450.0;
            let active_height = 150.0;
            let active_rect = Size::new(active_width, active_height)
                .to_rect()
                .with_origin(Point::new(
                    size.width / 2.0 - active_width / 2.0,
                    size.height / 2.0 - active_height / 2.0 + paint_rect.y0,
                ));
            let shadow_width = 5.0;
            ctx.blurred_rect(
                active_rect,
                shadow_width,
                data.config
                    .get_color_unchecked(LapceTheme::LAPCE_DROPDOWN_SHADOW),
            );
            ctx.fill(
                active_rect,
                data.config
                    .get_color_unchecked(LapceTheme::PANEL_BACKGROUND),
            );

            let input_height = 35.0;
            let rect = Size::new(0.0, 0.0)
                .to_rect()
                .with_origin(rect.center())
                .inflate(active_width / 2.0 - 10.0, input_height / 2.0);
            ctx.fill(
                rect,
                data.config
                    .get_color_unchecked(LapceTheme::EDITOR_BACKGROUND),
            );
            ctx.stroke(
                rect,
                data.config.get_color_unchecked(LapceTheme::LAPCE_BORDER),
                1.0,
            );
            KeyMap {
                key: keys.clone(),
                modes: keymap.modes.clone(),
                when: keymap.when.clone(),
                command: keymap.command.clone(),
            }
            .paint(ctx, rect.center(), Alignment::Center, &data.config);

            if let Some(cmd) = data.keypress.commands.get(&keymap.command) {
                let text = ctx
                    .text()
                    .new_text_layout(
                        cmd.palette_desc.clone().unwrap_or(cmd.cmd.clone()),
                    )
                    .font(FontFamily::SYSTEM_UI, 13.0)
                    .text_color(
                        data.config
                            .get_color_unchecked(LapceTheme::EDITOR_FOREGROUND)
                            .clone(),
                    )
                    .build()
                    .unwrap();
                let text_size = text.size();
                let rect_center = active_rect.center();
                let text_center = Point::new(
                    rect_center.x,
                    active_rect.y0
                        + (active_rect.height() / 2.0 - input_height / 2.0) / 2.0,
                );
                ctx.draw_text(
                    &text,
                    Point::new(
                        text_center.x - text_size.width / 2.0,
                        text_center.y - text_size.height / 2.0,
                    ),
                );
            }

            let center = active_rect.center()
                + (
                    active_width / 4.0,
                    input_height / 2.0
                        + (active_height / 2.0 - input_height / 2.0) / 2.0,
                );
            let text = ctx
                .text()
                .new_text_layout("Save".to_string())
                .font(FontFamily::SYSTEM_UI, 13.0)
                .text_color(
                    data.config
                        .get_color_unchecked(LapceTheme::EDITOR_FOREGROUND)
                        .clone(),
                )
                .build()
                .unwrap();
            let text_size = text.size();
            ctx.draw_text(
                &text,
                Point::new(
                    center.x - text_size.width / 2.0,
                    center.y - text_size.height / 2.0,
                ),
            );

            self.keymap_confirm = Size::new(0.0, 0.0)
                .to_rect()
                .with_origin(center)
                .inflate(50.0, 15.0);
            ctx.stroke(
                self.keymap_confirm,
                data.config.get_color_unchecked(LapceTheme::LAPCE_BORDER),
                1.0,
            );

            let center = active_rect.center()
                + (
                    -active_width / 4.0,
                    input_height / 2.0
                        + (active_height / 2.0 - input_height / 2.0) / 2.0,
                );
            let text = ctx
                .text()
                .new_text_layout("Cancel".to_string())
                .font(FontFamily::SYSTEM_UI, 13.0)
                .text_color(
                    data.config
                        .get_color_unchecked(LapceTheme::EDITOR_FOREGROUND)
                        .clone(),
                )
                .build()
                .unwrap();
            let text_size = text.size();
            ctx.draw_text(
                &text,
                Point::new(
                    center.x - text_size.width / 2.0,
                    center.y - text_size.height / 2.0,
                ),
            );
            self.keymap_cancel = Size::new(0.0, 0.0)
                .to_rect()
                .with_origin(center)
                .inflate(50.0, 15.0);
            ctx.stroke(
                self.keymap_cancel,
                data.config.get_color_unchecked(LapceTheme::LAPCE_BORDER),
                1.0,
            );
        }
    }
}

pub struct LapceKeymapHeader {}

impl LapceKeymapHeader {
    pub fn new() -> Self {
        Self {}
    }
}

impl Widget<LapceTabData> for LapceKeymapHeader {
    fn event(
        &mut self,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut LapceTabData,
        env: &Env,
    ) {
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &LapceTabData,
        env: &Env,
    ) {
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &LapceTabData,
        data: &LapceTabData,
        env: &Env,
    ) {
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &LapceTabData,
        env: &Env,
    ) -> Size {
        Size::new(bc.max().width, 40.0)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &LapceTabData, env: &Env) {
        let size = ctx.size();
        let keypress_width = 200.0;

        let text_layout = ctx
            .text()
            .new_text_layout("Command".to_string())
            .font(FontFamily::SYSTEM_UI, 14.0)
            .default_attribute(TextAttribute::Weight(FontWeight::BOLD))
            .text_color(
                data.config
                    .get_color_unchecked(LapceTheme::EDITOR_FOREGROUND)
                    .clone(),
            )
            .build()
            .unwrap();
        let text_size = text_layout.size();
        ctx.draw_text(
            &text_layout,
            Point::new(10.0, (size.height - text_size.height) / 2.0),
        );

        let text_layout = ctx
            .text()
            .new_text_layout("Keybinding".to_string())
            .font(FontFamily::SYSTEM_UI, 14.0)
            .default_attribute(TextAttribute::Weight(FontWeight::BOLD))
            .text_color(
                data.config
                    .get_color_unchecked(LapceTheme::EDITOR_FOREGROUND)
                    .clone(),
            )
            .build()
            .unwrap();
        let text_size = text_layout.size();
        ctx.draw_text(
            &text_layout,
            Point::new(
                size.width / 2.0 - keypress_width + 10.0,
                (size.height - text_size.height) / 2.0,
            ),
        );

        let text_layout = ctx
            .text()
            .new_text_layout("When".to_string())
            .font(FontFamily::SYSTEM_UI, 14.0)
            .default_attribute(TextAttribute::Weight(FontWeight::BOLD))
            .text_color(
                data.config
                    .get_color_unchecked(LapceTheme::EDITOR_FOREGROUND)
                    .clone(),
            )
            .build()
            .unwrap();
        let text_size = text_layout.size();
        ctx.draw_text(
            &text_layout,
            Point::new(
                size.width / 2.0
                    + 10.0
                    + if data.config.lapce.modal {
                        keypress_width
                    } else {
                        0.0
                    },
                (size.height - text_size.height) / 2.0,
            ),
        );

        if data.config.lapce.modal {
            let text_layout = ctx
                .text()
                .new_text_layout("Modes".to_string())
                .font(FontFamily::SYSTEM_UI, 14.0)
                .default_attribute(TextAttribute::Weight(FontWeight::BOLD))
                .text_color(
                    data.config
                        .get_color_unchecked(LapceTheme::EDITOR_FOREGROUND)
                        .clone(),
                )
                .build()
                .unwrap();
            let text_size = text_layout.size();
            ctx.draw_text(
                &text_layout,
                Point::new(
                    size.width / 2.0 + 10.0,
                    (size.height - text_size.height) / 2.0,
                ),
            );
        }

        let x = size.width / 2.0 - keypress_width;
        ctx.stroke(
            Line::new(Point::new(x, 0.0), Point::new(x, size.height)),
            data.config.get_color_unchecked(LapceTheme::LAPCE_BORDER),
            1.0,
        );
        let x = size.width / 2.0;
        ctx.stroke(
            Line::new(Point::new(x, 0.0), Point::new(x, size.height)),
            data.config.get_color_unchecked(LapceTheme::LAPCE_BORDER),
            1.0,
        );
        if data.config.lapce.modal {
            let x = size.width / 2.0 + keypress_width;
            ctx.stroke(
                Line::new(Point::new(x, 0.0), Point::new(x, size.height)),
                data.config.get_color_unchecked(LapceTheme::LAPCE_BORDER),
                1.0,
            );
        }
    }
}
