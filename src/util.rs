/// # Panics
/// - If the indices are the same
#[inline(always)]
pub fn get_two_mut<'a, T>(
    values: &'a mut [T],
    index1: usize,
    index2: usize,
) -> Option<(&'a mut T, &'a mut T)> {
    assert_ne!(index1, index2);

    let ptr1 = values.get_mut(index1)? as *mut T;
    let ptr2 = values.get_mut(index2)? as *mut T;

    // SAFETY: We just checked that the keys are different, so the pointers are different.
    unsafe { Some((&mut *ptr1, &mut *ptr2)) }
}

/// # Safety
/// - The indices must be different
/// - The indices must be within the bounds of the slice
#[inline(always)]
pub unsafe fn get_two_mut_unchecked<'a, T>(
    values: &'a mut [T],
    index1: usize,
    index2: usize,
) -> (&'a mut T, &'a mut T)
{
    debug_assert_ne!(index1, index2);
    debug_assert!(index1 < values.len());
    debug_assert!(index2 < values.len());

    let ptr1 = values.as_mut_ptr().add(index1);
    let ptr2 = values.as_mut_ptr().add(index2);

    unsafe { (&mut *ptr1, &mut *ptr2) }
}
