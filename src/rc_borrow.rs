use std::rc::Rc;
use std::ops::Deref;

/// RcBorrow is a run-time life-time checker.
/// 
/// It's a wrapper around a reference that can live longer than the reference, but panics
/// if the wrapper still has a living reference when the wrapper is dropped.
pub struct RcBorrow<'a, T> {
    rc: Rc<Borrow<T>>,
    _t: &'a T
}

impl<'a, T> RcBorrow<'a, T> {
    pub fn new(t: &'a T) -> Self {
        Self {_t: t, rc: Rc::new(Borrow(t as *const T))}
    }
    pub fn get(&self)->Rc<Borrow<T>> {
        self.rc.clone()
    }
}

impl<'a, T> Deref for RcBorrow<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &**(self.rc.as_ref())
    }
}

impl<'a, T> Drop for RcBorrow<'a, T> {
    fn drop(&mut self) {
        if Rc::strong_count(&self.rc) != 1 {
            panic!("someone still has the Rc, need to abort to avoid memory unsafety");
        }
    }
}

pub struct Borrow<T>(*const T);

impl<T> Deref for Borrow<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.0 }
    }
}

#[test]
fn test_rc_borrow() {
    let x: u32 = 123;
    {
        let rcb = RcBorrow::new(&x);
        let r=rcb.get();
        assert_eq!(**r, 123);
    }
}


#[test]
fn test_rc_borrow_deref() {
    let x: u32 = 123;
    {
        let rcb = RcBorrow::new(&x);
        assert_eq!(*rcb, 123);
    }
}


#[test]
#[should_panic]
fn test_rc_borrow_panic() {
    let x: u32 = 123;
    let mut _evil: Option<Rc<Borrow<u32>>> = None;
    {
        let rcb = RcBorrow::new(&x);
        assert_eq!(**rcb.get(), 123);
        _evil = Some(rcb.get());
    }
}
