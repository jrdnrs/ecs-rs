use std::{collections::HashMap, fmt::Debug, hash::Hash};

pub fn get_two_mut<'a, T, K>(
    map: &'a mut HashMap<K, T, ahash::RandomState>,
    key1: &K,
    key2: &K,
) -> Option<(&'a mut T, &'a mut T)>
where
    K: Eq + Hash + Debug,
{
    assert_ne!(key1, key2);

    let ptr1 = map.get_mut(key1)? as *mut T;
    let ptr2 = map.get_mut(key2)? as *mut T;

    // SAFETY: We just checked that the keys are different, so the pointers are different.
    unsafe { Some((&mut *ptr1, &mut *ptr2)) }
}

/// # Safety
/// - The keys must be valid for this map
/// - The keys must be different
pub unsafe fn get_two_mut_unchecked<'a, T, K>(
    map: &'a mut HashMap<K, T, ahash::RandomState>,
    key1: &K,
    key2: &K,
) -> (&'a mut T, &'a mut T)
where
    K: Eq + Hash + Debug,
{
    debug_assert_ne!(key1, key2);
    debug_assert!(map.contains_key(key1));
    debug_assert!(map.contains_key(key2));

    let ptr1 = map.get_mut(key1).unwrap_unchecked() as *mut T;
    let ptr2 = map.get_mut(key2).unwrap_unchecked() as *mut T;

    unsafe { (&mut *ptr1, &mut *ptr2) }
}
