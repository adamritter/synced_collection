use crate::{proxy::{Proxy, Listener}};
use std::hash::Hash;
use crate::SetModifyMessage;

pub type HashMap<K:Eq+Hash,V:Eq> = Proxy<std::collections::HashMap<K,V>, SetModifyMessage<(K, V)>>;

impl<K:Eq,V:Eq> HashMap<K,V> where K : Clone, K: Hash, V:Clone {
    pub fn new()->Self {
        Self::from(std::collections::HashMap::new())
    }
    pub fn set(&mut self, key: K, value: V) {
        let old_value=self.get_object().get(&key);
        if old_value.is_some() {
            let old_value_copy=old_value.unwrap().clone();
            self.process_and_send(SetModifyMessage::Update((key, old_value_copy), (key, value)))
        } else {
            *old_value.unwrap()=value;
            self.process_and_send(SetModifyMessage::Insert((key, value)));
        }
    }
    pub fn clear(&mut self) {
        self.process_and_send(SetModifyMessage::Clear());
    }
    pub fn remove(&mut self, key: &K) {
        let old_value=self.get(key).clone();
        if old_value.is_some() {
            self.process_and_send(SetModifyMessage::Remove((key.clone(), *old_value.unwrap())));
        }
    }
    pub fn len(&self)->usize {
        self.get_object().len()
    }
    pub fn get(&self, key: &K)->Option<&V> {
        self.get_object().get(key)
    }
}

impl<K:Eq+Hash, V:Eq> Listener for std::collections::HashMap<K,V> {
    type ModifyMessage = SetModifyMessage<(K, V)>;
    fn process(&mut self, message: Self::ModifyMessage) {
        match message {
            SetModifyMessage::Insert((key, value))=>{self.insert(key, value);},
            SetModifyMessage::Clear()=>{self.clear();},
            SetModifyMessage::Remove((key, value))=>{self.remove(&key);}
            SetModifyMessage::Update((old_key, old_value), (new_key, new_value))=>{
                self.remove(&old_key);
                self.insert(new_key, new_value);
            },
        }
    }
}
