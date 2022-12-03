use piet_scene::{
    kurbo::{Affine, Point, Rect},
    Brush, Color, Scene, SceneBuilder,
};
use piet_wgsl::{util::RenderContext, Renderer, Result};
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub async fn start_graphics_thread() -> Result<()> {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(1044, 800))
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();

    let render_cx = RenderContext::new().await?;
    let size = window.inner_size();
    let mut surface = render_cx.create_surface(&window, size.width, size.height);
    let mut renderer = Renderer::new(&render_cx.device)?;

    // let mut simple_text = simple_text::SimpleText::new();

    let mut current_frame = 0usize;
    let mut scene = Scene::default();

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::KeyboardInput { input, .. } => {}
            WindowEvent::Resized(size) => {
                render_cx.resize_surface(&mut surface, size.width, size.height);
                window.request_redraw();
            }
            _ => {}
        },
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        Event::RedrawRequested(_) => {
            current_frame += 1;

            let width = surface.config.width;
            let height = surface.config.height;

            let mut builder = SceneBuilder::for_scene(&mut scene);

            // Fill background color
            let bg_rect = Rect::from_origin_size(Point::new(0.0, 0.0), (width as _, height as _));
            builder.fill(
                piet_scene::Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(Color::rgb8(128, 128, 128)),
                None,
                &bg_rect,
            );

            builder.finish();
            let surface_texture = surface
                .surface
                .get_current_texture()
                .expect("failed to get surface texture");
            renderer
                .render_to_surface(
                    &render_cx.device,
                    &render_cx.queue,
                    &scene,
                    &surface_texture,
                    width,
                    height,
                )
                .expect("failed to render to surface");
            surface_texture.present();
            render_cx.device.poll(wgpu::Maintain::Wait);
        }
        _ => {}
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
