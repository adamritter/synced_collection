use std::{cell::RefCell};

pub struct MessageListeners<'a> {
    listeners: RefCell<Vec<Box<dyn FnMut(()) + 'a>>>
}

impl<'a> MessageListeners<'a> {
    pub fn new()->Self {
        MessageListeners { listeners: RefCell::new(Vec::new()) }
    }
    pub fn listen(&self, f: impl FnMut(())+'a) {
        self.listeners.borrow_mut().push(Box::new(f));
    }
    pub fn send(&self, message: ()) {
        for listener in self.listeners.borrow_mut().iter_mut() {
            (*listener)(message.clone());
        }
    }
    pub fn map(&self, f: impl Fn(())->())->MessageListeners<'a> {
        let r = MessageListeners::new();
        self.listeners().listen(|m|  r.send(f(m)));
        r
    }
}

pub trait MessageListenersInterface {
    fn listeners(&self)->&MessageListeners;
}

impl<'a> MessageListenersInterface for MessageListeners<'a> {
    fn listeners<'b>(&'b self)->&'a MessageListeners<'b> {
        self
    }
}


#[test]
fn test_message_listeners_map0() {
    let ml = MessageListeners::new();
    let f = |m: ()| {};
    let g = |m: ()| m;
    ml.map(g).listen(f);
    ml.send(());
}

#[test]
fn test_message_listeners_map2() {
    let ml = MessageListeners::new();
    let f = |m: ()| {};
    let g = |m: ()| m;
    let temp_variable=ml.map(g);
    temp_variable.listen(f);
    ml.send(());
}
