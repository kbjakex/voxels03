pub mod input;

/**
 * # Safety
 * See [`std::alloc::GlobalAlloc::alloc_zeroed`].
 */
#[inline(always)]
pub unsafe fn boxed_zeroed<T>() -> Box<T> {
    let layout = std::alloc::Layout::new::<T>();
    let mem = unsafe { std::alloc::alloc_zeroed(layout) };
    if mem.is_null() {
        std::alloc::handle_alloc_error(layout);
    }
    unsafe { Box::from_raw(mem.cast::<T>()) }
}