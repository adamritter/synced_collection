pub trait Listener {
    type ModifyMessage;
    fn process(&mut self, modify_message: Self::ModifyMessage);
}
pub struct Proxy<O, M> where O : Listener<ModifyMessage = M> {
    object: O,
    followers: Vec<Box<dyn FnMut(M)>>
}

impl<M,O:Clone> Proxy<O, M> where O : Listener<ModifyMessage = M>, M : Clone {
    pub fn process_and_send(&mut self, modify_message: M) {
        self.object.process(modify_message.clone());
        for follower in self.followers {
            (*follower)(modify_message.clone());
        }
    }
    pub fn from(object: O)->Self { 
        Self { object, followers: Vec::new() } 
    }
    fn add_follower<F:FnMut(M)+ 'static> (
        &mut self, follower: F) {
        let boxed = Box::new(follower);
        self.followers.push(boxed);
    }
    pub fn get_object(&self)->&O {
        &self.object
    }
}
