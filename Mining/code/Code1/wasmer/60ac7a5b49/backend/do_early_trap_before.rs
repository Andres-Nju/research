    unsafe fn do_early_trap(&self, data: Box<dyn Any>) -> ! {
        throw_any(Box::leak(data));
    }
