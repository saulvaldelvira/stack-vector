//! A vector-like object allocated on the stack
//!
//! # Example
//! ```
//! use stack_vector::StackVec;
//!
//! let mut sv = StackVec::<i32, 10>::new();
//!
//! sv.push(1);
//!
//! if false {
//!     sv.push(2);
//! }
//!
//! sv.push(3);
//!
//! if true {
//!     sv.push(4);
//! }
//!
//! assert_eq!(sv.as_slice(), &[1, 3, 4]);
//! ```

#![no_std]

use core::iter::Peekable;
use core::mem::{self, ManuallyDrop, MaybeUninit};
use core::ops::{Deref, DerefMut, RangeBounds};
use core::ptr::{self, NonNull};

use drain::Drain;

mod drain;

/// A [Vec]-like wrapper for an array.
///
/// This struct allows to push and pop to an array,
/// treating it like a vector, but with no heap allocations.
pub struct StackVec<T, const CAP: usize> {
    inner: [MaybeUninit<T>; CAP],
    length: usize,
}

impl<T, const CAP: usize> StackVec<T, CAP> {
    /// Creates a new empty StackVec
    #[inline]
    pub const fn new() -> Self {
        Self {
            inner: [const { MaybeUninit::uninit() }; CAP],
            length: 0,
        }
    }

    /// Creates a new StackVec, filled with copies of the given value
    ///
    /// # Example
    /// ```
    /// use stack_vector::StackVec;
    ///
    /// let v = StackVec::<i32, 5>::filled(0);
    /// assert_eq!(v.as_slice(), &[0, 0, 0, 0, 0]);
    /// ```
    #[inline(always)]
    pub fn filled(val: T) -> Self
    where
        T: Clone,
    {
        Self::generate(|| val.clone())
    }

    /// Creates a new StackVec, filling it using the given generator function
    ///
    /// # Example
    /// ```
    /// use stack_vector::StackVec;
    ///
    /// let mut n = 0;
    /// let v = StackVec::<i32, 5>::generate(|| {
    ///     n += 1;
    ///     n
    /// });
    ///
    /// assert_eq!(v.len(), 5);
    /// assert_eq!(v.as_slice(), &[1, 2, 3, 4, 5]);
    /// ```
    pub fn generate<Gen>(mut generator: Gen) -> Self
    where
        Gen: FnMut() -> T,
    {
        let mut s = Self::new();
        for _ in 0..CAP {
            unsafe {
                /* SAFETY: We only call this function CAP
                 * times, so it's never gonna fail */
                s.push_unchecked(generator());
            }
        }
        s
    }

    /// Creates a new StackVec from the given array of T
    ///
    /// # Example
    /// ```
    /// use stack_vector::StackVec;
    ///
    /// let v = StackVec::from_array([1, 2, 3, 4, 5]);
    ///
    /// assert_eq!(v.len(), 5);
    /// assert_eq!(v.as_slice(), &[1, 2, 3, 4, 5]);
    /// ```
    pub const fn from_array(arr: [T; CAP]) -> Self {
        /* We can't transmute the array due to rust's limitations.
         * We need to wrap the array into a ManuallyDrop, to avoid
         * T's Drop to be called twice. */
        let arr = ManuallyDrop::new(arr);
        let inner = unsafe {
            /* SAFETY: T and ManualyDrop<T> have the same size and alignment */
            mem::transmute_copy(&arr)
        };
        Self { inner, length: CAP }
    }

    /// Pushes an element in the StackVec without checking bounds.
    ///
    /// # Safety
    /// Caller must ensure that the StackVec has room for the element
    #[inline]
    pub unsafe fn push_unchecked(&mut self, val: T) {
        unsafe {
            self.as_mut_ptr().add(self.length).write(val);
        }
        self.length += 1;
    }

    /// Pushes an element into this StackVec, panicking if there is no space left.
    ///
    /// # Panics
    /// - If the StackVec is full
    #[inline]
    pub fn push(&mut self, val: T) {
        if self.try_push(val).is_err() {
            panic!("Attemp to push beyond the capacity of the array")
        }
    }

    /// Attempts to push an element into this StackVec.
    ///
    /// # Errors
    /// - If the StackVec if full, returns back the element
    ///   inside an Err variant.
    pub fn try_push(&mut self, val: T) -> Result<(), T> {
        if self.length >= CAP {
            Err(val)
        } else {
            /* SAFETY: We've just checked that the buffer can
             * hold the element */
            unsafe { self.push_unchecked(val) };
            Ok(())
        }
    }

    /// Pushes all the elements from the iterator into this StackVec.
    #[inline]
    pub fn extend_from_iter<I>(&mut self, it: I)
    where
        I: IntoIterator<Item = T>,
    {
        for elem in it.into_iter() {
            self.push(elem)
        }
    }

    /// Attempts to push all the elements from the iterator into this StackVec.
    ///
    /// # Errors
    /// If the iterator yields more elements that we can push, returns the
    /// iterator (turned into a [Peekable]) as an Err variant
    pub fn try_extend_from_iter<I>(
        &mut self,
        it: I,
    ) -> Result<(), Peekable<<I as IntoIterator>::IntoIter>>
    where
        I: IntoIterator<Item = T>,
    {
        let mut it = it.into_iter().peekable();
        while it.peek().is_some() {
            if self.length >= CAP {
                return Err(it);
            }
            unsafe {
                /* SAFETY:
                 * 1) In the while condition, we've checked that the
                 *    iterator has a next element.
                 *
                 * 2) In the condition above, we check that there's room
                 *    for this element
                 * */
                let elem = it.next().unwrap_unchecked();
                self.push_unchecked(elem)
            }
        }
        Ok(())
    }

    /// Removes the ith element of the StackVec, and returns it.
    ///
    /// # Safety
    /// - i must be within bounds [0, [Self::len])
    pub unsafe fn remove_unchecked(&mut self, i: usize) -> T {
        /* SAFETY: self.inner[i] is initialized, thus reading
         * from this pointer is safe */
        let ret = unsafe { self.inner[i].assume_init_read() };

        let ptr = self.inner.as_mut_ptr();

        unsafe {
            /* SAFETY: Elements [i + 1, len) are within bounds
             * for the buffer, and can be copied over */
            ptr::copy(ptr.add(i + 1), ptr.add(i), self.length - i - 1);
        }
        self.length -= 1;
        ret
    }

    /// Removes the ith element of the StackVec, and returns it.
    /// If the index is out of bounds, returns None
    pub fn remove(&mut self, i: usize) -> Option<T> {
        if i <= self.length {
            unsafe { Some(self.remove_unchecked(i)) }
        } else {
            None
        }
    }

    /// Removes the last element of the StackVec, and returns it.
    /// If empty, returns None
    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        self.remove(self.length)
    }

    /// Returns an slice of T's from this StackVec, with all
    /// the currently allocated elements.
    pub const fn as_slice(&self) -> &[T] {
        let (slice, _) = self.inner.split_at(self.length);
        /* SAFETY:
         * - The items in range 0..self.len are initialized
         * - MaybeUninit<T> and T have the same memory layout and alignment */
        unsafe { mem::transmute::<&[MaybeUninit<T>], &[T]>(slice) }
    }

    /// Returns a mutable slice of T's from this StackVec, with
    /// all the currently allocated elements.
    pub const fn as_slice_mut(&mut self) -> &mut [T] {
        let (slice, _) = self.inner.split_at_mut(self.length);
        /* SAFETY: Same as as_slice */
        unsafe { mem::transmute::<&mut [MaybeUninit<T>], &mut [T]>(slice) }
    }

    /// Clears all the elements in this StackVec
    pub fn clear(&mut self) {
        let ptr = self.as_slice_mut() as *mut [T];
        unsafe {
            /* SAFETY
             * We set length to 0 before calling drop_in_place.
             * In case a Drop call fails, we're good.
             */
            self.length = 0;
            ptr::drop_in_place(ptr);
        }
    }

    /// Returns this StackVec's buffer as a *const T.
    #[inline(always)]
    pub const fn as_ptr(&self) -> *const T {
        self.inner.as_ptr() as *const T
    }

    /// Returns this StackVec's buffer as a *mut T.
    #[inline(always)]
    pub const fn as_mut_ptr(&mut self) -> *mut T {
        self.inner.as_mut_ptr() as *mut T
    }

    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<'_, T, CAP> {
        use core::ops::Bound;

        let start = match range.start_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => *i + 1,
            Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            Bound::Included(i) => *i + 1,
            Bound::Excluded(i) => *i,
            Bound::Unbounded => self.length,
        };

        /* SAFETY: A reference is always non null */
        let sv = unsafe { NonNull::new_unchecked(self) };

        let iter = self.as_slice()[start..end].iter();
        let len = end - start;

        Drain::new(sv, iter, start, len)
    }

    /// Returns the capacity of this StackVec.
    /// This is just a convenience function, since the
    /// capacity is a const generic argument.
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    /// Returns the remaining capacity of this StackVec.
    /// This is, how many more elements can we store in it.
    #[inline(always)]
    pub const fn remaining_capacity(&self) -> usize {
        CAP - self.length
    }

    /// Returns the length of this StackVec, this is, the
    /// number of elements "pushed" into it.
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.length
    }

    /// Returns true if the length is 0
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Returns true if no more elements can be pushed into this StackVec
    #[inline(always)]
    pub const fn is_full(&self) -> bool {
        self.length == CAP
    }
}

impl<T, const CAP: usize> Deref for StackVec<T, CAP> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const CAP: usize> DerefMut for StackVec<T, CAP> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_slice_mut()
    }
}

impl<T, const CAP: usize> Default for StackVec<T, CAP> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const CAP: usize> From<[T; CAP]> for StackVec<T, CAP> {
    #[inline(always)]
    fn from(value: [T; CAP]) -> Self {
        StackVec::from_array(value)
    }
}

impl<T, const CAP: usize> Drop for StackVec<T, CAP> {
    fn drop(&mut self) {
        if mem::needs_drop::<T>() {
            self.clear();
        }
    }
}

impl<T: Clone, const CAP: usize> Clone for StackVec<T, CAP> {
    fn clone(&self) -> Self {
        let mut inner = [const { MaybeUninit::uninit() }; CAP];
        let src = self.inner.as_ptr();
        let dst = inner.as_mut_ptr();
        unsafe {
            ptr::copy(src, dst, self.length);
        }
        Self {
            inner,
            length: self.length,
        }
    }
}

impl<T: PartialEq, const CAP: usize> PartialEq for StackVec<T, CAP> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice().iter().eq(other.as_slice().iter())
    }
}

impl<T: PartialOrd, const CAP: usize> PartialOrd for StackVec<T, CAP> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.as_slice().iter().partial_cmp(other.as_slice().iter())
    }
}
