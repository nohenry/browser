use drawing_context::DrawingContext;
use vello::{kurbo::Size, Scene, SceneBuilder};
use vello::{util::RenderContext, Renderer, Result};
use simple_text::SimpleText;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub use vello;

pub mod simple_text;

pub mod drawing_context;

pub async fn start_graphics_thread(draw: impl Fn(&mut DrawingContext) + 'static) -> Result<()> {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(1044, 800))
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();

    let mut render_cx = RenderContext::new();
    let size = window.inner_size();
    let mut surface = render_cx.create_surface(&window, size.width, size.height).await?;
    let mut device_handle = &render_cx.devices[surface.dev_id];
    let mut renderer = Renderer::new(&device_handle.device)?;

    // let mut simple_text = simple_text::SimpleText::new();

    let mut scene = Scene::default();

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
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
            let width = surface.config.width;
            let height = surface.config.height;

            let mut device_handle = &render_cx.devices[surface.dev_id];

            let mut dctx = DrawingContext {
                builder: SceneBuilder::for_scene(&mut scene),
                text: SimpleText::new(),
                size: Size::new(width as _, height as _),
            };

            draw(&mut dctx);

            dctx.builder.finish();
            let surface_texture = surface
                .surface
                .get_current_texture()
                .expect("failed to get surface texture");
            renderer
                .render_to_surface(
                    &device_handle.device,
                    &device_handle.queue,
                    &scene,
                    &surface_texture,
                    width,
                    height,
                )
                .expect("failed to render to surface");
            surface_texture.present();
            // render_cx.device.poll(wgpu::Maintain::Wait);
            device_handle.device.poll(wgpu::MaintainBase::Wait);
        }
        _ => {}
    });
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
