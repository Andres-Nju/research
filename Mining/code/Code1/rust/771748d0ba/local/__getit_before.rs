            unsafe fn __getit() -> $crate::option::Option<
                &'static $crate::cell::UnsafeCell<
                    $crate::option::Option<$t>>>
            {
                #[cfg(target_arch = "wasm32")]
                static __KEY: $crate::thread::__StaticLocalKeyInner<$t> =
                    $crate::thread::__StaticLocalKeyInner::new();

                #[thread_local]
                #[cfg(all(target_thread_local, not(target_arch = "wasm32")))]
                static __KEY: $crate::thread::__FastLocalKeyInner<$t> =
                    $crate::thread::__FastLocalKeyInner::new();

                #[cfg(all(not(target_thread_local), not(target_arch = "wasm32")))]
                static __KEY: $crate::thread::__OsLocalKeyInner<$t> =
                    $crate::thread::__OsLocalKeyInner::new();

                __KEY.get()
            }

            unsafe {
                $crate::thread::LocalKey::new(__getit, __init)
            }
        }
    };
    ($(#[$attr:meta])* $vis:vis $name:ident, $t:ty, $init:expr) => {
        $(#[$attr])* $vis const $name: $crate::thread::LocalKey<$t> =
            __thread_local_inner!(@key $(#[$attr])* $vis $name, $t, $init);
    }
}

/// An error returned by [`LocalKey::try_with`](struct.LocalKey.html#method.try_with).
#[stable(feature = "thread_local_try_with", since = "1.26.0")]
pub struct AccessError {
    _private: (),
}

#[stable(feature = "thread_local_try_with", since = "1.26.0")]
impl fmt::Debug for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AccessError").finish()
    }
}

#[stable(feature = "thread_local_try_with", since = "1.26.0")]
impl fmt::Display for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt("already destroyed", f)
    }
}

impl<T: 'static> LocalKey<T> {
    #[doc(hidden)]
    #[unstable(feature = "thread_local_internals",
               reason = "recently added to create a key",
               issue = "0")]
    pub const unsafe fn new(inner: unsafe fn() -> Option<&'static UnsafeCell<Option<T>>>,
                            init: fn() -> T) -> LocalKey<T> {
        LocalKey {
            inner,
            init,
        }
    }

    /// Acquires a reference to the value in this TLS key.
    ///
    /// This will lazily initialize the value if this thread has not referenced
    /// this key yet.
    ///
    /// # Panics
    ///
    /// This function will `panic!()` if the key currently has its
    /// destructor running, and it **may** panic if the destructor has
    /// previously been run for this thread.
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn with<F, R>(&'static self, f: F) -> R
                      where F: FnOnce(&T) -> R {
        self.try_with(f).expect("cannot access a TLS value during or \
                                 after it is destroyed")
    }

    unsafe fn init(&self, slot: &UnsafeCell<Option<T>>) -> &T {
        // Execute the initialization up front, *then* move it into our slot,
        // just in case initialization fails.
        let value = (self.init)();
        let ptr = slot.get();

        // note that this can in theory just be `*ptr = Some(value)`, but due to
        // the compiler will currently codegen that pattern with something like:
        //
        //      ptr::drop_in_place(ptr)
        //      ptr::write(ptr, Some(value))
        //
        // Due to this pattern it's possible for the destructor of the value in
        // `ptr` (e.g. if this is being recursively initialized) to re-access
        // TLS, in which case there will be a `&` and `&mut` pointer to the same
        // value (an aliasing violation). To avoid setting the "I'm running a
        // destructor" flag we just use `mem::replace` which should sequence the
        // operations a little differently and make this safe to call.
        mem::replace(&mut *ptr, Some(value));

        (*ptr).as_ref().unwrap()
    }

    /// Acquires a reference to the value in this TLS key.
    ///
    /// This will lazily initialize the value if this thread has not referenced
    /// this key yet. If the key has been destroyed (which may happen if this is called
    /// in a destructor), this function will return a `ThreadLocalError`.
    ///
    /// # Panics
    ///
    /// This function will still `panic!()` if the key is uninitialized and the
    /// key's initializer panics.
    #[stable(feature = "thread_local_try_with", since = "1.26.0")]
    pub fn try_with<F, R>(&'static self, f: F) -> Result<R, AccessError>
    where
        F: FnOnce(&T) -> R,
    {
        unsafe {
            let slot = (self.inner)().ok_or(AccessError {
                _private: (),
            })?;
            Ok(f(match *slot.get() {
                Some(ref inner) => inner,
                None => self.init(slot),
            }))
        }
    }
}

/// On some platforms like wasm32 there's no threads, so no need to generate
/// thread locals and we can instead just use plain statics!
#[doc(hidden)]
#[cfg(target_arch = "wasm32")]
pub mod statik {
    use cell::UnsafeCell;
    use fmt;

    pub struct Key<T> {
        inner: UnsafeCell<Option<T>>,
    }

    unsafe impl<T> ::marker::Sync for Key<T> { }

    impl<T> fmt::Debug for Key<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.pad("Key { .. }")
        }
    }

    impl<T> Key<T> {
        pub const fn new() -> Key<T> {
            Key {
                inner: UnsafeCell::new(None),
            }
        }

        pub unsafe fn get(&self) -> Option<&'static UnsafeCell<Option<T>>> {
            Some(&*(&self.inner as *const _))
        }
    }
}

#[doc(hidden)]
#[cfg(target_thread_local)]
pub mod fast {
    use cell::{Cell, UnsafeCell};
    use fmt;
    use mem;
    use ptr;
    use sys::fast_thread_local::{register_dtor, requires_move_before_drop};

    pub struct Key<T> {
        inner: UnsafeCell<Option<T>>,

        // Metadata to keep track of the state of the destructor. Remember that
        // these variables are thread-local, not global.
        dtor_registered: Cell<bool>,
        dtor_running: Cell<bool>,
    }

    impl<T> fmt::Debug for Key<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.pad("Key { .. }")
        }
    }

    impl<T> Key<T> {
        pub const fn new() -> Key<T> {
            Key {
                inner: UnsafeCell::new(None),
                dtor_registered: Cell::new(false),
                dtor_running: Cell::new(false)
            }
        }

        pub unsafe fn get(&self) -> Option<&'static UnsafeCell<Option<T>>> {
            if mem::needs_drop::<T>() && self.dtor_running.get() {
                return None
            }
            self.register_dtor();
            Some(&*(&self.inner as *const _))
        }

        unsafe fn register_dtor(&self) {
            if !mem::needs_drop::<T>() || self.dtor_registered.get() {
                return
            }

            register_dtor(self as *const _ as *mut u8,
                          destroy_value::<T>);
            self.dtor_registered.set(true);
        }
    }

    unsafe extern fn destroy_value<T>(ptr: *mut u8) {
        let ptr = ptr as *mut Key<T>;
        // Right before we run the user destructor be sure to flag the
        // destructor as running for this thread so calls to `get` will return
        // `None`.
        (*ptr).dtor_running.set(true);

        // Some implementations may require us to move the value before we drop
        // it as it could get re-initialized in-place during destruction.
        //
        // Hence, we use `ptr::read` on those platforms (to move to a "safe"
        // location) instead of drop_in_place.
        if requires_move_before_drop() {
            ptr::read((*ptr).inner.get());
        } else {
            ptr::drop_in_place((*ptr).inner.get());
        }
    }
}

#[doc(hidden)]
pub mod os {
    use cell::{Cell, UnsafeCell};
    use fmt;
    use marker;
    use ptr;
    use sys_common::thread_local::StaticKey as OsStaticKey;

    pub struct Key<T> {
        // OS-TLS key that we'll use to key off.
        os: OsStaticKey,
        marker: marker::PhantomData<Cell<T>>,
    }

    impl<T> fmt::Debug for Key<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.pad("Key { .. }")
        }
    }

    unsafe impl<T> ::marker::Sync for Key<T> { }

    struct Value<T: 'static> {
        key: &'static Key<T>,
        value: UnsafeCell<Option<T>>,
    }

    impl<T: 'static> Key<T> {
        pub const fn new() -> Key<T> {
            Key {
                os: OsStaticKey::new(Some(destroy_value::<T>)),
                marker: marker::PhantomData
            }
        }

        pub unsafe fn get(&'static self) -> Option<&'static UnsafeCell<Option<T>>> {
            let ptr = self.os.get() as *mut Value<T>;
            if !ptr.is_null() {
                if ptr as usize == 1 {
                    return None
                }
                return Some(&(*ptr).value);
            }

            // If the lookup returned null, we haven't initialized our own
            // local copy, so do that now.
            let ptr: Box<Value<T>> = box Value {
                key: self,
                value: UnsafeCell::new(None),
            };
            let ptr = Box::into_raw(ptr);
            self.os.set(ptr as *mut u8);
            Some(&(*ptr).value)
        }
