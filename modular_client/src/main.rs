use anyhow::{anyhow, Result};
use femtovg::{renderer::OpenGl, Canvas, Color, Paint, Path, Transform2D};
use glutin::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder,
};
use modular_client::{
    app::App,
    ui::{
        box_constraints::BoxConstraints,
        components::{grid::Grid, tile::Tile},
        context::{Context, Theme},
        edge_insets::EdgeInsets,
        size::Size,
        widget::Widget,
        widgets::{
            align::{Align, Alignment},
            clip::Clip,
            container::Container,
            custom_paint::CustomPaint,
            flex::{Axis, Flex, FlexRule},
            padding::Padding,
            painted_box::PaintedBox,
            sized_box::SizedBox,
            stack::Stack,
            text::Text,
            transformed_box::TransformedBox,
        },
    },
};
use rand::prelude::*;
use std::{
    thread,
    time::{Duration, Instant},
};
use stretch::style::FlexDirection;

pub fn main() -> Result<()> {
    let window_size = PhysicalSize::new(1000, 700);
    let el = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_inner_size(window_size)
        .with_resizable(true)
        .with_title("Modular");

    let windowed_context = ContextBuilder::new()
        .with_vsync(true)
        .build_windowed(wb, &el)?;
    let windowed_context = unsafe { windowed_context.make_current().unwrap() };
    let renderer = OpenGl::new(|s| windowed_context.get_proc_address(s) as *const _)
        .expect("Cannot create renderer");
    let mut canvas = Canvas::new(renderer).expect("Cannot create canvas");
    canvas.set_size(
        window_size.width as u32,
        window_size.height as u32,
        windowed_context.window().scale_factor() as f32,
    );
    let mut rng = rand::thread_rng();
    let mut make_nodes = move || {
        (0..100)
            .into_iter()
            .map(|_| {
                (
                    (rng.next_u32() % 40, rng.next_u32() % 40),
                    Color::rgb(rng.gen(), rng.gen(), rng.gen()),
                )
            })
            .collect()
    };
    let mut nodes: Vec<_> = make_nodes();

    // let font = canvas.add_font_mem(include_bytes!(
    //     "assets/Fira_Code_v5.2/ttf/FiraCode-Regular.ttf"
    // ))?;
    let mut now = Instant::now();
    let t = 0.0;
    let max_t = 3.0;
    let mut app = App::new(&mut canvas).unwrap();
    let mut context = Context {
        dpi_factor: windowed_context.window().scale_factor() as f32,
        theme: Theme {
            background: Color::rgb(0, 0, 0),
            f_high: Color::rgb(255, 255, 255),
            f_med: Color::rgb(119, 119, 119),
            f_low: Color::rgb(68, 68, 68),
            f_inv: Color::rgb(0, 0, 0),
            b_high: Color::rgb(238, 238, 238),
            b_med: Color::rgb(114, 222, 194),
            b_low: Color::rgb(68, 68, 68),
            b_inv: Color::rgb(255, 181, 69),
        },
    };
    el.run(move |event, _window, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::LoopDestroyed => return,
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(physical_size) => {
                    windowed_context.resize(*physical_size);
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::ModifiersChanged(modifiers) => {
                    app.handle_modifier_change(*modifiers);
                }
                WindowEvent::ReceivedCharacter(c) => {
                    app.handle_char_recieved(*c);
                    windowed_context.window().request_redraw();
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(keycode),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    app.handle_key_press(*keycode);
                    windowed_context.window().request_redraw();
                }
                _ => (),
            },
            Event::RedrawRequested(_) => {
                let dpi_factor = windowed_context.window().scale_factor() as f32;
                let size = windowed_context.window().inner_size();
                context.dpi_factor = dpi_factor;

                canvas.set_size(size.width as u32, size.height as u32, dpi_factor);
                canvas.clear_rect(
                    0,
                    0,
                    size.width as u32,
                    size.height as u32,
                    context.theme.background,
                );
                canvas.reset_transform();
                canvas.scale(dpi_factor, dpi_factor);

                // let padding = EdgeInsets::all(20.0);
                // let colors = [
                //     Color::rgb(0, 0, 255),
                //     Color::rgb(0, 255, 0),
                //     Color::rgb(255, 0, 0),
                // ];

                // let mut container: Box<dyn Widget> = Align::new(
                //     Alignment::center(),
                //     Text::new("ABCVVV".to_owned()).with_fill(paint).package(),
                // );
                // for color in colors.repeat(1) {
                //     container = PaintedBox::new()
                //         .with_fill(Paint::color(color))
                //         .with_child(Padding::new(container, padding))
                //         .package();
                // }

                // let mut container = Flex::new(Axis::Vertical)
                //     .with_child(
                //         FlexRule::Flex(1.0),
                //         PaintedBox::new()
                //             .with_fill(Paint::color(Color::black()))
                //             .with_child(Align::new(
                //                 Alignment::center(),
                //                 Text::new("It Worksq!".to_owned())
                //                     .with_stroke({
                //                         let mut paint = Paint::color(Color::white());
                //                         paint.set_font_size(100.0);
                //                         paint.set_font(&[font]);
                //                         paint.set_line_width(1.0);
                //                         paint
                //                     }),
                //             )),
                //     )
                //     .with_child(
                //         FlexRule::Fixed(5.0),
                //         PaintedBox::new()
                //             .with_fill(Paint::color(Color::white())),
                //     )
                //     .with_child(
                //         FlexRule::Fixed(50.0),
                //         PaintedBox::new()
                //             .with_fill(Paint::color(Color::black())),
                //     );
                // let mut container = Stack::new(vec![
                //     {
                //         let x_dim = 100.0;
                //         let y_dim = 100.0;
                //         let dash_on_ratio = 1.0;
                //         let dash_off_ratio = 1.0;
                //         let x_count = 4;
                //         let y_count = 4;
                //         Grid::new(
                //             x_dim,
                //             y_dim,
                //             dash_on_ratio,
                //             dash_off_ratio,
                //             x_count,
                //             y_count,
                //         )
                //     },
                //     // {
                //     //     let x_dim = 400.0;
                //     //     let y_dim = 400.0;
                //     //     let dash_on_ratio = 1.0;
                //     //     let dash_off_ratio = 6.0;
                //     //     let x_count = 1;
                //     //     let y_count = 1;
                //     //     Grid::new(
                //     //         x_dim,
                //     //         y_dim,
                //     //         dash_on_ratio,
                //     //         dash_off_ratio,
                //     //         x_count,
                //     //         y_count,
                //     //     )
                //     // },
                // ]);

                let grid = {
                    let dim = 20.0;
                    // let size = Size::new(20.0, 20.0);
                    // let painter =
                    //     move |_size: Size, canvas: &mut Canvas<OpenGl>, context: &Context| {
                    //         let mut path = Path::new();
                    //         path.move_to(0.0, dim / 2.0);
                    //         path.line_to(dim, dim / 2.0);
                    //         path.move_to(dim / 2.0, 0.0);
                    //         path.line_to(dim / 2.0, dim);
                    //         let mut paint = Paint::color(context.theme.b_low);
                    //         paint.set_line_width(2.0);
                    //         canvas.stroke_path(&mut path, paint);
                    //     };
                    // let mut grid: Vec<Box<dyn Widget>> = vec![];

                    // grid.extend(nodes.iter().map(|((x, y), color)| {
                    //     let mut paint = Paint::color(*color);
                    //     paint.set_line_width(3.0);
                    //     TransformedBox::new(
                    //         Transform2D::new_translation(
                    //             *x as f32 * size.width,
                    //             *y as f32 * size.height,
                    //         ),
                    //         SizedBox::new(size).with_child(Padding::new(
                    //             EdgeInsets::all(5.0),
                    //             PaintedBox::new().with_stroke(paint),
                    //         )),
                    //     )
                    //     .pack()
                    // }));
                    // grid
                    let mut flex = Flex::new(Axis::Horizontal);
                    for _ in 0..4 {
                        let mut inner_flex = Flex::new(Axis::Vertical);
                        for i in 0..30 {
                            let text = Align::new(
                                Alignment::center_left(),
                                Text::new(i.to_string()).with_fill({
                                    let mut paint = Paint::color(context.theme.f_high);
                                    paint.set_font_size(15.0);
                                    paint.set_font(&[app.font]);
                                    paint
                                }),
                            );
                            if i % 2 == 0 {
                                inner_flex = inner_flex.with_child(FlexRule::Fixed(dim), text);
                            } else {
                                inner_flex = inner_flex.with_child(
                                    FlexRule::Fixed(dim),
                                    PaintedBox::new()
                                        .with_fill(Paint::color(context.theme.b_low))
                                        .with_child(text),
                                );
                            }
                        }
                        flex = flex.with_child(
                            FlexRule::Fixed(100.0),
                            Stack::new_from_boxed(vec![
                                Grid::new(f32::INFINITY, dim, 1.0, 0.0, 1, 1).pack(),
                                inner_flex.pack(),
                            ]),
                        );
                    }
                    flex
                };

                let mut container = Flex::new(Axis::Vertical)
                    .with_child(FlexRule::Flex(1.0), Clip::new(grid))
                    .with_child(
                        FlexRule::Fixed(5.0),
                        PaintedBox::new().with_fill(Paint::color(context.theme.b_low)),
                    )
                    .with_child(FlexRule::Fixed(50.0), app.build_prompt(&context));

                container.layout(
                    BoxConstraints::loose(Size::new(
                        size.width as f32 / dpi_factor,
                        size.height as f32 / dpi_factor,
                    )),
                    &mut canvas,
                    &context,
                );
                container.paint(&mut canvas, &context);
                // let mut paint = Paint::color(Color::white());
                // paint.set_font_size(100.0 * dpi_factor as f32);
                // paint.set_font(&[font]);
                // canvas
                //     .stroke_text(0.0, 120.0 * dpi_factor as f32, "text2 ==", paint)
                //     .unwrap();
                // let elapsed = start.elapsed().as_secs_f32();
                // let now = Instant::now();
                // let dt = (now - prevt).as_secs_f32();
                // prevt = now;

                // perf.update(dt);

                // draw_baselines(&mut canvas, &fonts, 5.0, 50.0, font_size);
                // draw_alignments(&mut canvas, &fonts, 120.0, 200.0, font_size);
                // draw_paragraph(&mut canvas, &fonts, x, y, font_size, LOREM_TEXT);
                // draw_inc_size(&mut canvas, &fonts, 300.0, 10.0);

                // draw_complex(&mut canvas, 300.0, 340.0, font_size);

                // draw_stroked(&mut canvas, &fonts, size.width as f32 - 200.0, 100.0);
                // draw_gradient_fill(&mut canvas, &fonts, size.width as f32 - 200.0, 180.0);
                // draw_image_fill(
                //     &mut canvas,
                //     &fonts,
                //     size.width as f32 - 200.0,
                //     260.0,
                //     image_id,
                //     elapsed,
                // );

                // let mut paint = Paint::color(Color::hex("B7410E"));
                // paint.set_font(&[fonts.bold]);
                // paint.set_text_baseline(Baseline::Top);
                // paint.set_text_align(Align::Right);
                // let _ = canvas.fill_text(
                //     size.width as f32 - 10.0,
                //     10.0,
                //     format!(
                //         "Scroll to increase / decrease font size. Current: {}",
                //         font_size
                //     ),
                //     paint,
                // );

                // canvas.save();
                // canvas.reset();
                // perf.render(&mut canvas, 5.0, 5.0);
                // canvas.restore();

                canvas.flush();
                windowed_context.swap_buffers().unwrap();
            }
            // Event::MainEventsCleared => windowed_context.window().request_redraw(),
            _ => (),
        }
    });
}
