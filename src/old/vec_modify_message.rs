use crate::proxy::{Proxy, Listener};

#[derive(Clone)]
pub enum VecModifyMessage<T> {
    ReplaceAll(Vec<T>),
    Insert(usize, T),
    Set(usize, T),
    Push(T),
    Clear(),
    Remove(usize)
}

impl<T> Proxy<Vec<T>, VecModifyMessage<T>> where T:Clone {
    pub fn set(&mut self, vec: Vec<T>) {
        self.process_and_send(VecModifyMessage::ReplaceAll(vec));
    }
    pub fn insert(&mut self, i: usize, el: T) {
        self.process_and_send(VecModifyMessage::Insert(i, el));
    }
    pub fn push(&mut self, el: T) {
        self.process_and_send(VecModifyMessage::Push(el));
    }
    pub fn clear(&mut self) {
        self.process_and_send(VecModifyMessage::Clear());
    }
    pub fn remove(&mut self, i: usize) {
        self.process_and_send(VecModifyMessage::Remove(i));
    }
    pub fn len(&self)->usize {
        self.get_object().len()
    }
    pub fn get(&self, i:usize)->Option<&T> {
        self.get_object().get(i)
    }
    pub fn index(&self, i:usize)->&T {
        &self.get_object()[i]
    }
}

impl<T:Clone> Listener for Vec<T> {
    type ModifyMessage = VecModifyMessage<T>;
    fn process(&mut self, message: Self::ModifyMessage) {
        match message {
            VecModifyMessage::ReplaceAll(vec)=> { _= std::mem::replace(self, vec); },
            VecModifyMessage::Insert(i, elem)=>{self.insert(i, elem);},
            VecModifyMessage::Set(i, elem)=>{self[i]=elem;},
            VecModifyMessage::Push(elem)=>{self.push(elem);},
            VecModifyMessage::Clear()=>{self.clear();},
            VecModifyMessage::Remove(i)=>{self.remove(i);}
        }
    }
}

#[test]
fn test_set() {
    // let synced_vec =  SyncedVec::new();
    // synced_vec.insert(0, "hello");
    // let other_synced_vec=SyncedVec::new();
    // CloneSync::connect(synced_vec, other_synced_vec);
    // other_synced_vec.push("world");
    // assert_eq!("world", synced_vec[1]);
}