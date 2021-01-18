use std::{
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use femtovg::{renderer::OpenGl, Canvas, Color, Paint, Path};
use glutin::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder,
};
use modular_client::ui::{
    box_constraints::BoxConstraints,
    components::grid::Grid,
    context::{Context, Theme},
    edge_insets::EdgeInsets,
    size::Size,
    widget::Widget,
    widgets::{
        align::{Align, Alignment},
        container::Container,
        custom_paint::CustomPaint,
        flex::{Axis, Flex, FlexRule},
        padding::Padding,
        painted_box::PaintedBox,
        stack::Stack,
        text::Text,
    },
};

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

    let font = canvas.add_font_mem(include_bytes!(
        "assets/Fira_Code_v5.2/ttf/FiraCode-Regular.ttf"
    ))?;
    let mut now = Instant::now();
    let t = 0.0;
    let max_t = 3.0;
    el.run(move |event, window, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::LoopDestroyed => return,
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(physical_size) => {
                    windowed_context.resize(*physical_size);
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(keycode),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    if *keycode == VirtualKeyCode::W {
                        // y -= 0.1;
                    }

                    if *keycode == VirtualKeyCode::S {
                        // y += 0.1;
                    }

                    if *keycode == VirtualKeyCode::A {
                        // x -= 0.1;
                    }

                    if *keycode == VirtualKeyCode::D {
                        // x += 0.1;
                    }
                }
                WindowEvent::MouseWheel {
                    device_id: _,
                    delta,
                    ..
                } => match delta {
                    glutin::event::MouseScrollDelta::LineDelta(_, y) => {
                        // font_size = font_size + *y / 2.0;
                        // font_size = font_size.max(2.0);
                    }
                    _ => (),
                },
                _ => (),
            },
            Event::RedrawRequested(_) => {
                let dpi_factor = windowed_context.window().scale_factor() as f32;
                let size = windowed_context.window().inner_size();
                let ref context = Context {
                    dpi_factor,
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
                let mut container = Stack::new(vec![
                    {
                        let x_dim = 100.0;
                        let y_dim = 100.0;
                        let dash_on_ratio = 1.0;
                        let dash_off_ratio = 1.0;
                        let x_count = 4;
                        let y_count = 4;
                        Grid::new(
                            x_dim,
                            y_dim,
                            dash_on_ratio,
                            dash_off_ratio,
                            x_count,
                            y_count,
                        )
                    },
                    // {
                    //     let x_dim = 400.0;
                    //     let y_dim = 400.0;
                    //     let dash_on_ratio = 1.0;
                    //     let dash_off_ratio = 6.0;
                    //     let x_count = 1;
                    //     let y_count = 1;
                    //     Grid::new(
                    //         x_dim,
                    //         y_dim,
                    //         dash_on_ratio,
                    //         dash_off_ratio,
                    //         x_count,
                    //         y_count,
                    //     )
                    // },
                ]);

                container.layout(
                    BoxConstraints::loose(Size::new(
                        size.width as f32 / dpi_factor,
                        size.height as f32 / dpi_factor,
                    )),
                    &mut canvas,
                    context,
                );
                container.paint(&mut canvas, context);
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
