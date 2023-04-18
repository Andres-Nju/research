fn test_as_mut() {
    unsafe {
        let p: *mut isize = null_mut();
        assert!(p.as_mut() == None);

        let q: *mut isize = &mut 2;
        assert!(q.as_mut().unwrap() == &mut 2);

        // Lifetime inference
        let mut u = 2isize;
        {
            let p = &mut u as *mut isize;
            assert!(p.as_mut().unwrap() == &mut 2);
        }

        // Pointers to unsized types -- slices
        let s: &mut [u8] = &mut [1, 2, 3];
        let ms: *mut [u8] = s;
        assert_eq!(ms.as_mut(), Some(&mut [1, 2, 3]));

        let mz: *mut [u8] = &mut [];
        assert_eq!(mz.as_mut(), Some(&mut [][..]));

        let nms: *mut [u8] = null_mut::<[u8; 3]>();
        assert_eq!(nms.as_mut(), None);

        // Pointers to unsized types -- trait objects
        let mi: *mut dyn ToString = &mut 3;
        assert!(mi.as_mut().is_some());

        let nmi: *mut dyn ToString = null_mut::<isize>();
        assert!(nmi.as_mut().is_none());
    }
}
