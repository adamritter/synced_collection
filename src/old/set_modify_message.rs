use std::{collections::HashSet, hash::Hash};

use crate::proxy::{Proxy, Listener};

#[derive(Clone)]
pub enum SetModifyMessage<T> {
    Insert(T),
    Clear(),
    Remove(T),
    Update(T,T)
}

impl<T:Eq> Proxy<HashSet<T>, SetModifyMessage<T>> where T : Clone, T: Hash {
    pub fn insert(&mut self, el: T) {
        self.process_and_send(SetModifyMessage::Insert(el));
    }
    pub fn clear(&mut self) {
        self.process_and_send(SetModifyMessage::Clear());
    }
    pub fn remove(&mut self, el: &T) {
        self.process_and_send(SetModifyMessage::Remove(el.clone()));
    }
    pub fn update(&mut self, old: &T, new: T) {
        self.process_and_send(SetModifyMessage::Update(old.clone(), new));
    }
    pub fn len(&self)->usize {
        self.get_object().len()
    }
    pub fn get(&self, el: &T)->Option<&T> {
        self.get_object().get(el)
    }
}

impl<T:Eq+Hash> Listener for HashSet<T> {
    type ModifyMessage = SetModifyMessage<T>;
    fn process(&mut self, message: Self::ModifyMessage) {
        match message {
            SetModifyMessage::Insert(elem)=>{self.insert(elem);},
            SetModifyMessage::Clear()=>{self.clear();},
            SetModifyMessage::Remove(elem)=>{self.remove(&elem);}
            SetModifyMessage::Update(old, new)=>{self.remove(&old); self.insert(new);},
        }
    }
}
