    fn test_syscall_sol_alloc_free() {
        let config = Config::default();

        // large alloc
        {
            prepare_mockup!(
                invoke_context,
                transaction_context,
                program_id,
                bpf_loader::id(),
            );
            let mut heap = AlignedMemory::new_with_size(100, HOST_ALIGN);
            let mut memory_mapping = MemoryMapping::new::<UserError>(
                vec![
                    MemoryRegion::default(),
                    MemoryRegion::new_readonly(&[], ebpf::MM_PROGRAM_START),
                    MemoryRegion::new_writable_gapped(&mut [], ebpf::MM_STACK_START, 4096),
                    MemoryRegion::new_writable(heap.as_slice_mut(), ebpf::MM_HEAP_START),
                    MemoryRegion::new_writable(&mut [], ebpf::MM_INPUT_START),
                ],
                &config,
            )
            .unwrap();
            invoke_context
                .set_syscall_context(
                    true,
                    true,
                    vec![],
                    Rc::new(RefCell::new(BpfAllocator::new(heap, ebpf::MM_HEAP_START))),
                )
                .unwrap();
            let mut syscall = SyscallAllocFree {
                invoke_context: Rc::new(RefCell::new(&mut invoke_context)),
            };
            let mut result: Result<u64, EbpfError<BpfError>> = Ok(0);
            syscall.call(100, 0, 0, 0, 0, &mut memory_mapping, &mut result);
            assert_ne!(result.unwrap(), 0);
            let mut result: Result<u64, EbpfError<BpfError>> = Ok(0);
            syscall.call(100, 0, 0, 0, 0, &mut memory_mapping, &mut result);
            assert_eq!(result.unwrap(), 0);
            let mut result: Result<u64, EbpfError<BpfError>> = Ok(0);
            syscall.call(u64::MAX, 0, 0, 0, 0, &mut memory_mapping, &mut result);
            assert_eq!(result.unwrap(), 0);
        }

        // many small unaligned allocs
        {
            prepare_mockup!(
                invoke_context,
                transaction_context,
                program_id,
                bpf_loader::id(),
            );
            let mut heap = AlignedMemory::new_with_size(100, HOST_ALIGN);
            let mut memory_mapping = MemoryMapping::new::<UserError>(
                vec![
                    MemoryRegion::default(),
                    MemoryRegion::new_readonly(&[], ebpf::MM_PROGRAM_START),
                    MemoryRegion::new_writable_gapped(&mut [], ebpf::MM_STACK_START, 4096),
                    MemoryRegion::new_writable(heap.as_slice_mut(), ebpf::MM_HEAP_START),
                    MemoryRegion::new_writable(&mut [], ebpf::MM_INPUT_START),
                ],
                &config,
            )
            .unwrap();
            invoke_context
                .set_syscall_context(
                    false,
                    true,
                    vec![],
                    Rc::new(RefCell::new(BpfAllocator::new(heap, ebpf::MM_HEAP_START))),
                )
                .unwrap();
            let mut syscall = SyscallAllocFree {
                invoke_context: Rc::new(RefCell::new(&mut invoke_context)),
            };
            for _ in 0..100 {
                let mut result: Result<u64, EbpfError<BpfError>> = Ok(0);
                syscall.call(1, 0, 0, 0, 0, &mut memory_mapping, &mut result);
                assert_ne!(result.unwrap(), 0);
            }
            let mut result: Result<u64, EbpfError<BpfError>> = Ok(0);
            syscall.call(100, 0, 0, 0, 0, &mut memory_mapping, &mut result);
            assert_eq!(result.unwrap(), 0);
        }

        // many small aligned allocs
        {
            prepare_mockup!(
                invoke_context,
                transaction_context,
                program_id,
                bpf_loader::id(),
            );
            let mut heap = AlignedMemory::new_with_size(100, HOST_ALIGN);
            let mut memory_mapping = MemoryMapping::new::<UserError>(
                vec![
                    MemoryRegion::default(),
                    MemoryRegion::new_readonly(&[], ebpf::MM_PROGRAM_START),
                    MemoryRegion::new_writable_gapped(&mut [], ebpf::MM_STACK_START, 4096),
                    MemoryRegion::new_writable(heap.as_slice_mut(), ebpf::MM_HEAP_START),
                    MemoryRegion::new_writable(&mut [], ebpf::MM_INPUT_START),
                ],
                &config,
            )
            .unwrap();
            invoke_context
                .set_syscall_context(
                    true,
                    true,
                    vec![],
                    Rc::new(RefCell::new(BpfAllocator::new(heap, ebpf::MM_HEAP_START))),
                )
                .unwrap();
            let mut syscall = SyscallAllocFree {
                invoke_context: Rc::new(RefCell::new(&mut invoke_context)),
            };
            for _ in 0..12 {
                let mut result: Result<u64, EbpfError<BpfError>> = Ok(0);
                syscall.call(1, 0, 0, 0, 0, &mut memory_mapping, &mut result);
                assert_ne!(result.unwrap(), 0);
            }
            let mut result: Result<u64, EbpfError<BpfError>> = Ok(0);
            syscall.call(100, 0, 0, 0, 0, &mut memory_mapping, &mut result);
            assert_eq!(result.unwrap(), 0);
        }

        // aligned allocs

        fn aligned<T>() {
            prepare_mockup!(
                invoke_context,
                transaction_context,
                program_id,
                bpf_loader::id(),
            );
            let mut heap = AlignedMemory::new_with_size(100, HOST_ALIGN);
            let config = Config::default();
            let mut memory_mapping = MemoryMapping::new::<UserError>(
                vec![
                    MemoryRegion::default(),
                    MemoryRegion::new_readonly(&[], ebpf::MM_PROGRAM_START),
                    MemoryRegion::new_writable_gapped(&mut [], ebpf::MM_STACK_START, 4096),
                    MemoryRegion::new_writable(heap.as_slice_mut(), ebpf::MM_HEAP_START),
                    MemoryRegion::new_writable(&mut [], ebpf::MM_INPUT_START),
                ],
                &config,
            )
            .unwrap();
            invoke_context
                .set_syscall_context(
                    true,
                    true,
                    vec![],
                    Rc::new(RefCell::new(BpfAllocator::new(heap, ebpf::MM_HEAP_START))),
                )
                .unwrap();
            let mut syscall = SyscallAllocFree {
                invoke_context: Rc::new(RefCell::new(&mut invoke_context)),
            };
            let mut result: Result<u64, EbpfError<BpfError>> = Ok(0);
            syscall.call(
                size_of::<u8>() as u64,
                0,
                0,
                0,
                0,
                &mut memory_mapping,
                &mut result,
            );
            let address = result.unwrap();
            assert_ne!(address, 0);
            assert_eq!(
                (address as *const u8 as usize).wrapping_rem(align_of::<u8>()),
                0
            );
        }
        aligned::<u8>();
        aligned::<u16>();
        aligned::<u32>();
        aligned::<u64>();
        aligned::<u128>();
    }
