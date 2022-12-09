use std::{cell::RefCell, rc::Rc};

pub struct MessageListeners<'listener, M> {
    listeners: RefCell<Vec<Option<Box<dyn FnMut(M) + 'listener>>>>
}

impl<'listener, M:Clone+'static> MessageListeners<'listener, M> {
    pub fn new()->Self {
        MessageListeners { listeners: RefCell::new(Vec::new()) }
    }

    /// This method takes a function object and adds it to the vector of listeners.
    pub fn listen(&self, f: impl FnMut(M)+'listener)->usize {
        self.listeners.borrow_mut().push(Some(Box::new(f)));
        self.listeners.borrow().len()-1
    }
    
    /// This method sends a message to all listeners in the vector.
    pub fn send(&self, message: M) {
        for listener in self.listeners.borrow_mut().iter_mut() {
            if let Some(listener2)=listener {
                (*listener2)(message.clone());
            }
        }
    }

    pub fn cancel(&self, i: usize) {
        self.listeners.borrow_mut()[i] = None;
    }
}


pub trait MessageListenersInterface<'listener, M:Clone+'static> : Sized {
    fn listeners(&self)->&MessageListeners<'listener, M>;
    fn listen(&self, f: impl FnMut(M)+'listener) {
        println!("MessageListenersInterface::listen");
        MessageListeners::listen(self.listeners(), f);
    }
     fn map<M2:Clone+'static>(&self, f: impl Fn(M)->M2 + 'listener)->Rc<MessageListeners<'listener, M2>> {
        let r = Rc::new(MessageListeners::new());
        let rclone=r.clone();
        self.listen(move |m|  rclone.send(f(m)));
        r
    }
    fn filter(&self, f: impl Fn(&M)->bool + 'listener)->Rc<MessageListeners<'listener, M>> {
        let r = Rc::new(MessageListeners::new());
        let rclone=r.clone();
        self.listeners().listen(move |m| if f(&m) { rclone.send(m)});
        r
    }
}

impl<'listener, M:Clone+'static> MessageListenersInterface<'listener, M> for MessageListeners<'listener, M> {
    fn listeners(&self)->&MessageListeners<'listener, M> {
        self
    }
}

#[test]
fn test_message_listeners() {
    // Arrange
    let mut messages = Vec::new();
    {
        let ml = MessageListeners::new();
        let f = |m: i32| {
        messages.push(m);
        };

    // Act
    ml.listen(f);
    ml.send(1);
    ml.send(2);
    ml.send(3);
    }

  // Assert
  assert_eq!(*messages, vec![1, 2, 3]);
}

#[test]
fn test_message_listeners_map() {
//   Arrange
  let mut messages: Vec<i32> = Vec::new();
  {
    let ml = MessageListeners::new();
    let f = |m: i32| {
        messages.push(m);
    };
    let g = |m: i32| m * 2;

    // There's a bug in the compiler here.  If you don't create a temporary variable, the compiler SEGFAULTS.
    ml.map(g).listen(f);
    ml.send(1);
    ml.send(2);
    ml.send(3);
  }

  // Assert
  assert_eq!(*messages, vec![2, 4, 6]);
}

#[test]
fn test_message_listeners_filter() {
  // Arrange
  let mut messages = Vec::new();
  {
    let ml = MessageListeners::new();
    let f = |m: i32| {
        messages.push(m);
    };
    let g = |m: &i32| *m % 2 == 0;

    // Act
    let gg=ml.filter(g);
    gg.listen(f);
    ml.send(1);
    ml.send(2);
    ml.send(3);
    ml.send(4);
    }

  // Assert
  assert_eq!(*messages, vec![2, 4]);
}
