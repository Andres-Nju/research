fn main() {
    env_logger::init().unwrap();
    let window = winit::WindowBuilder::new()
        .with_dimensions(1024, 768)
        .with_title("triangle (Low Level)".to_string())
        .build()
        .unwrap();

    // instantiate backend
    let instance = back::Instance::create();
    let physical_devices = instance.enumerate_adapters();
    let surface = instance.create_surface(&window);

    let queue_descs = physical_devices[0].get_queue_families().map(|family| { (family, family.num_queues()) });
    
    for device in &physical_devices {
        println!("{:?}", device.get_info());
    }

    // build a new device and associated command queues
    let (device, queues) = physical_devices[0].open(queue_descs);

    let mut swap_chain = surface.build_swapchain::<ColorFormat>(&queues[0]);

    'main: loop {
        for event in window.poll_events() {
            match event {
                winit::Event::KeyboardInput(_, _, Some(winit::VirtualKeyCode::Escape)) |
                winit::Event::Closed => break 'main,
                _ => {},
            }
