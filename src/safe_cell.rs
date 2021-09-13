use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};

pub struct SafeCell<E> {
    inner: UnsafeCell<E>
}

impl<E> SafeCell<E> {
    pub fn new(e: E) -> Self {
        Self {
            inner: UnsafeCell::new(e)
        }
    }
    /// Unsafely gets the wrapped object as reference.
    #[inline]
    pub(crate) fn get(&self) -> &E {
        unsafe { &*self.inner.get() }
    }

    /// Unsafely gets the wrapped object as mutable reference.
    #[inline]
    pub(crate) fn get_mut(&self) -> &mut E {
        unsafe { &mut *self.inner.get() }
    }
}

unsafe impl<E> Sync for SafeCell<E> { }
unsafe impl<E> Send for SafeCell<E> { }

impl<E> Deref for SafeCell<E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<E> DerefMut for SafeCell<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl<E> AsMut<E> for SafeCell<E> {
    fn as_mut(&mut self) -> &mut E {
        self.deref_mut()
    }
}

impl<E> AsRef<E> for SafeCell<E> {
    fn as_ref(&self) -> &E {
        self.deref()
    }
}

