use std::rc::Rc;

use crate::message_listeners::{MessageListeners, MessageListenersInterface};

#[derive(Clone, Debug)]
pub enum MultiSetModifyMessage<T:Clone> {
    InsertOne(T),
    RemoveOne(T),
}

pub type MultiSetMessageListeners<'a, T>=MessageListeners<'a, MultiSetModifyMessage<T>>;

impl<'a, T:Clone+'static> MultiSetMessageListeners<'a, T> {
    pub fn map_items<T2:Clone + 'static>(&self, f: impl Fn(T)->T2+'a)->Rc<MultiSetMessageListeners<'a, T2>> {
        self.map(move |message| {
            match message {
                MultiSetModifyMessage::InsertOne(item)=>MultiSetModifyMessage::InsertOne(f(item)),
                MultiSetModifyMessage::RemoveOne(item)=>MultiSetModifyMessage::RemoveOne(f(item)),
            }
        })
    }
}

#[test]
fn test_multi_set_message_listeners_map_items() {
    let ml = MultiSetMessageListeners::new();
    {
        let f = |_m: MultiSetModifyMessage<i32>| {};
        let g = |m: i32| m;
        let mi=ml.map_items(g);
        mi.listen(f);
        ml.send(MultiSetModifyMessage::InsertOne(1));
        ml.send(MultiSetModifyMessage::RemoveOne(2));
    }
}