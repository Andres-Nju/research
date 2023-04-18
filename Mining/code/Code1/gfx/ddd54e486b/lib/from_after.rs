    fn from(native: &Self::Native) -> Self;
    fn as_native(&self) -> &Self::Native;
}

pub type BufferPtr = NonNull<metal::MTLBuffer>;
pub type TexturePtr = NonNull<metal::MTLTexture>;
pub type SamplerPtr = NonNull<metal::MTLSamplerState>;

impl AsNative for BufferPtr {
    type Native = metal::BufferRef;
    #[inline]
    fn from(native: &metal::BufferRef) -> Self {
        unsafe { NonNull::new_unchecked(native.as_ptr()) }
    }
    #[inline]
    fn as_native(&self) -> &metal::BufferRef {
        unsafe { metal::BufferRef::from_ptr(self.as_ptr()) }
    }
}
