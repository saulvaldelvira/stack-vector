use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::mem;
use core::slice;
use core::ptr::{self, NonNull};

use crate::StackVec;

pub struct Drain<'a, T: 'a, const CAP: usize> {
    sv: NonNull<StackVec<T, CAP>>,
    iter: slice::Iter<'a, T>,
    start: usize,
    len: usize,
    _marker: PhantomData<&'a mut StackVec<T, CAP>>,
}

impl<'a, T: 'a, const CAP: usize> Drain<'a, T, CAP> {
    pub (super) fn new(
        sv: NonNull<StackVec<T, CAP>>,
        iter: slice::Iter<'a, T>,
        start: usize,
        len: usize,
    ) -> Self {
        Self {
            sv, iter, start, len, _marker: PhantomData
        }
    }
}

impl<T, const CAP: usize> Iterator for Drain<'_, T, CAP> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|p| unsafe { ptr::read(p) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T, const CAP: usize> DoubleEndedIterator for Drain<'_, T, CAP> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter
            .next_back()
            .map(|p| unsafe { ptr::read(p) })
    }
}

impl<T, const CAP: usize> FusedIterator for Drain<'_, T, CAP> { }

impl<T, const CAP: usize> ExactSizeIterator for Drain<'_, T, CAP> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<T, const CAP: usize> Drop for Drain<'_, T, CAP> {
    fn drop(&mut self) {
        if mem::needs_drop::<T>() {
            self.for_each(drop);
        }

        if self.len > 0 {
            unsafe {
                let sv = self.sv.as_mut();

                let dst = sv.as_mut_ptr().add(self.start);
                let src = dst.add(self.len);
                let n = sv.length - (self.start + self.len);

                ptr::copy(src, dst, n);

                sv.length -= self.len;
            }
        }
    }
}
