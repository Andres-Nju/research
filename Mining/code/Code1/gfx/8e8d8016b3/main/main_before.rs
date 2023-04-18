fn main() {
    #[cfg(target_arch = "wasm32")]
    console_log::init_with_level(log::Level::Debug).unwrap();
    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
    env_logger::init();

    #[cfg(not(target_arch = "wasm32"))]
    let event_loop = winit::event_loop::EventLoop::new();

    #[cfg(not(target_arch = "wasm32"))]
    let dpi = event_loop.primary_monitor().hidpi_factor();

    #[cfg(not(target_arch = "wasm32"))]
    let wb = winit::window::WindowBuilder::new()
        .with_min_inner_size(winit::dpi::LogicalSize::new(1.0, 1.0))
        .with_inner_size(winit::dpi::LogicalSize::from_physical(
            winit::dpi::PhysicalSize::new(DIMS.width as _, DIMS.height as _),
            dpi,
        ))
        .with_title("quad".to_string());
    // instantiate backend
    #[cfg(not(feature = "gl"))]
    let (_window, _instance, mut adapters, surface) = {
        let window = wb.build(&event_loop).unwrap();
        let instance = back::Instance::create("gfx-rs quad", 1)
            .expect("Failed to create an instance!");
        let surface = instance.create_surface(&window).expect("Failed to create a surface!");
        let adapters = instance.enumerate_adapters();
        (window, instance, adapters, surface)
    };
    #[cfg(feature = "gl")]
    let (window, mut adapters, surface) = {
        #[cfg(not(target_arch = "wasm32"))]
        let (window, surface) = {
            let builder =
                back::config_context(back::glutin::ContextBuilder::new(), ColorFormat::SELF, None)
                    .with_vsync(true);
            let windowed_context = builder.build_windowed(wb, &event_loop).unwrap();
            let (context, window) = unsafe {
                windowed_context
                    .make_current()
                    .expect("Unable to make context current")
                    .split()
            };
            let surface = back::Surface::from_context(context);
            (window, surface)
        };
        #[cfg(target_arch = "wasm32")]
        let (window, surface) = {
            let window = back::Window;
            let surface = back::Surface::from_window(&window);
            (window, surface)
        };

        let adapters = surface.enumerate_adapters();
        (window, adapters, surface)
    };

    for adapter in &adapters {
        println!("{:?}", adapter.info);
    }

    let adapter = adapters.remove(0);

    let mut renderer = Renderer::new(surface, adapter);

    #[cfg(target_arch = "wasm32")]
    renderer.render();

    #[cfg(not(target_arch = "wasm32"))]
    // It is important that the closure move captures the Renderer,
    // otherwise it will not be dropped when the event loop exits.
    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Wait;

        match event {
            winit::event::Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::CloseRequested => {
                    *control_flow = winit::event_loop::ControlFlow::Exit
                }
                winit::event::WindowEvent::KeyboardInput {
                    input:
                        winit::event::KeyboardInput {
                            virtual_keycode: Some(winit::event::VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => *control_flow = winit::event_loop::ControlFlow::Exit,
                winit::event::WindowEvent::Resized(dims) => {
                    println!("resized to {:?}", dims);
                    #[cfg(feature = "gl")]
                    {
                        let context = renderer.surface.context();
                        context.resize(dims.to_physical(window.hidpi_factor()));
                    }
                    renderer.dimensions = window::Extent2D {
                        width: (dims.width * dpi) as u32,
                        height: (dims.height * dpi) as u32,
                    };
                    renderer.recreate_swapchain();
                }
                _ => {}
            },
            winit::event::Event::EventsCleared => {
                renderer.render();
            }
            _ => {}
        }
    });
}
