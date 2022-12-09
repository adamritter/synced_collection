use std::{cell::RefCell, rc::Rc};

pub struct MessageListeners2<M> {
    listeners: RefCell<Vec<Box<dyn FnMut(M)>>>
}


impl<M:Clone+'static> MessageListeners2<M> {
    pub fn new()->Self {
        MessageListeners2 { listeners: RefCell::new(Vec::new()) }
    }
    pub fn listen<'a>(&self, f: impl FnMut(M)) {
        self.listeners.borrow_mut().push(Box::new(f));
    }
    pub fn send(&self, message: M) {
        for listener in self.listeners.borrow_mut().iter_mut() {
            (*listener)(message.clone());
        }
    }

}


pub trait MessageListeners2Interface<M:Clone+'static> : Sized {
    fn listeners(&self)->&MessageListeners2<M>;
    fn listen(&self, f: impl FnMut(M)) {
        println!("MessageListeners2Interface::listen");
        MessageListeners2::listen(self.listeners(), f);
    }
     fn map<M2:Clone+'static>(&self, f: impl Fn(M)->M2 )->Rc<MessageListeners2<M2>> {
        let r = Rc::new(MessageListeners2::new());
        let rclone=r.clone();
        self.listen(move |m|  rclone.send(f(m)));
        r
    }
    fn filter(&self, f: impl Fn(&M)->bool )->Rc<MessageListeners2<M>> {
        let r = Rc::new(MessageListeners2::new());
        let rclone=r.clone();
        self.listeners().listen(move |m| if f(&m) { rclone.send(m)});
        r
    }
}

impl<M:Clone+'static> MessageListeners2Interface<M> for MessageListeners2<M> {
    fn listeners(&self)->&MessageListeners2<M> {
        self
    }
}

#[test]
fn test_message_listeners() {
    // Arrange
    let mut messages = Vec::new();
    {
        let ml = MessageListeners2::new();
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
    let ml = MessageListeners2::new();
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
    let ml = MessageListeners2::new();
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
