use std::{collections::{HashSet, HashMap}, cell::RefCell, rc::{Rc, Weak}, marker::PhantomData};

use crate::{multi_set::{MultiSetModifyMessage, MultiSetMessageListeners}, message_listeners::{MessageListenersInterface, MessageListeners}, rc_borrow::RcBorrow};
use std::hash::Hash;

pub trait QuerableStreamingMultiMapReciever<'a, K:Eq+Hash+Clone + 'static,V:Eq+Hash+Clone+'static> :
     MessageListenersInterface<'a, MultiSetModifyMessage<(K,V)>>  {
    fn get(&self, key: &K)->HashSet<V>;
    fn get_one(&self, key: &K)->Option<V> {
        let set=self.get(key);
        if set.len()!=1 {
            None
        } else {
            Some(set.iter().next().unwrap().clone())
        }
    }
}


/// An interface that represents key-value pairs where multiple values can be
///   associated with a key, and a key-value pair can be inserted only once.
///
///  Requesting set of all values for a key is efficient, which makes joining
///   multiple QuerableStreamingMultiMaps on the same key efficient.
pub trait QuerableStreamingMultiMap<'a, K:Eq+Hash+Clone + 'static,V:Eq+Hash+Clone+'static> :
     MessageListenersInterface<'a, MultiSetModifyMessage<(K,V)>>  {
    fn get(&self, key: &K)->HashSet<V>;
    fn get_one(&self, key: &K)->Option<V> {
        let set=self.get(key);
        if set.len()!=1 {
            None
        } else {
            Some(set.iter().next().unwrap().clone())
        }
    }
    fn filter_item<'b, Allow: Fn(K,V)->bool>(&'b self, allow: Allow)->
            FilterQuerableStreamingMultiMap<'a, 'b, K, V, Self, Allow> {
        FilterQuerableStreamingMultiMap::new(self, allow)
    }
    fn filter_item_old<Allow: Fn(K,V)->bool+'a>(&'a self, allow: Allow)->
        Rc<MessageListeners<'a, MultiSetModifyMessage<(K,V)>>> {
        let f = move |m: &MultiSetModifyMessage<(K,V)>| {
            match m {
                MultiSetModifyMessage::InsertOne((k,v)) => allow(k.clone(), v.clone()),
                MultiSetModifyMessage::RemoveOne((k,v)) => allow(k.clone(), v.clone())
            }
        };
        self.filter(f)
    }
    fn map<T2:Clone+'static>(&'a self, f: impl Fn(K, V)->T2+'a)->Rc<MultiSetMessageListeners<'a, T2>> {
        self.listeners().map_items(move |(k, v)| f(k, v))
    }
    
    fn reversed(&'a self)->Rc<StreamingHashMultiMapWithCount<V, K>> {
        self.listeners().reversed()
    }
    fn group_by<K2: Eq+Hash+Clone+'static, V2: Eq+Hash+Clone+'static>(
            &'a self, f:impl Fn(K, V)->(K2, V2) + 'a)->Rc<StreamingHashMultiMapWithCount<K2,V2>> {
        self.listeners().group_by(f)
    }
}

// pub trait JoinMultiMap<'a, K:Eq+Hash+Clone + 'static,V:Eq+Hash+Clone+'static> :  QuerableStreamingMultiMap<'a,K,V> {
//     fn join<'b, 'c, 'd : 'a+'e, 'e, V2:Eq+Hash+Clone+'static, Source2: QuerableStreamingMultiMap<'e, K,V2>+'e>(
//         self: &'b  Self, source2: &'c Source2)
//         ->JoinQuerableStreamingMultiMap<'b, 'c, 'a, 'd, 'e, K, V, V2, Self, Source2> {
//             JoinQuerableStreamingMultiMap::new(self, source2)
//         }
// }

/// A data structure for both representing key-multiple values data and for group by + join
/// operations in memory.
/// 
/// A multimap with counts that can be an output of a group / group by operation for
///   streaming sets with a possibility of having a key value pair multiple times.
/// 
/// It also provides an interface that shows those pairs only once, thereby
/// it's a useful datastructure for joining multiple data sets on the same key.
pub struct StreamingHashMultiMapWithCount<'a, K: Eq+Hash+Clone+'static, V: Eq+Hash+Clone+'static> {
    listeners: MultiSetMessageListeners<'a, (K, V)>,
    data: RefCell<HashMap<K, HashMap<V, u64>>>
}

impl<'a, K: Eq+Hash+Clone+'static, V: Eq+Hash+Clone+'static> StreamingHashMultiMapWithCount<'a, K, V> {
    pub fn new()->Self {
        Self {listeners: MultiSetMessageListeners::new(), data: RefCell::new(HashMap::new())}
    }
}

impl<'a, K: Eq+Hash+Clone + 'static, V: Eq+Hash+Clone+'static>
     QuerableStreamingMultiMap<'a, K,V> for StreamingHashMultiMapWithCount<'a, K, V> {
    fn get(&self, key: &K)->HashSet<V> {
        let data = self.data.borrow();
        let values=data.get(key);
        if values.is_none() {
            return HashSet::new();
        } else {
            let mut r=HashSet::new();
            for key in values.unwrap().keys() {
                r.insert(key.clone());
            }
            return r;
        }
    }
    fn get_one(&self, key: &K)->Option<V> {
        let data = self.data.borrow();
        let values=data.get(key);
        if values.is_none() || values.unwrap().len()!=1 {
            return None;
        } else {
            return values.unwrap().keys().next().map(|v| v.clone());
        }
    }
}

impl <'a, K: Eq+Hash+Clone + 'static, V: Eq+Hash+Clone+'static> 
    MessageListenersInterface<'a, MultiSetModifyMessage<(K,V)>> for StreamingHashMultiMapWithCount<'a, K, V> {
    fn listeners(&self)->&crate::message_listeners::MessageListeners<'a, MultiSetModifyMessage<(K,V)>> {
        &self.listeners
    }
}

// impl<'a, K: Eq+Hash+Clone+'static, V: Eq+Hash+Clone+'static> JoinMultiMap<'a, K, V> for StreamingHashMultiMapWithCount<'a, K, V> {}

impl<'a, K: Eq+Hash+Clone+'static, V: Eq+Hash+Clone+'static> StreamingHashMultiMapWithCount<'a, K, V> {
    pub fn insert(&self, key: K, value: V) {
        let mut data = self.data.borrow_mut();
        let value_edit=data.get_mut(&key);
        if value_edit.is_none() {
            let mut value_hash_map=HashMap::new();
            value_hash_map.insert(value.clone(), 1);
            self.listeners.send(MultiSetModifyMessage::InsertOne((key.clone(), value.clone())));
           data.insert(key, value_hash_map);
        } else {
            let value_hash_map=value_edit.unwrap();
            let value_value = value_hash_map.get_mut(&value);
            if value_value.is_none() {
                value_hash_map.insert(value.clone(), 1);
                self.listeners.send(MultiSetModifyMessage::InsertOne((key, value)));
            } else {
                *value_value.unwrap()+=1;
            }
        }
    }
    pub fn remove(&self, key: K, value: V)->bool {
        let mut data = self.data.borrow_mut();
        let value_edit=data.get_mut(&key);
        if value_edit.is_none() {
            return false;
        } else {
            let value_hash_map=value_edit.unwrap();
            let value_value = value_hash_map.get_mut(&value);
            if value_value.is_none() {
                return false;
            } else {
                let vvu=value_value.unwrap();
                *vvu-=1;
                if *vvu==0 {
                    value_hash_map.remove(&value);
                    if value_hash_map.is_empty() {
                        data.remove(&key);
                    }
                    self.listeners.send(MultiSetModifyMessage::RemoveOne((key, value)));
                }
                return true;
            }
        }
    }
    pub fn set(&self, key: K, value: V) {
        let vs=self.get(&key);
        for v in vs {
            self.remove(key.clone(), v);
        }
        self.insert(key, value);
    }
}
 
pub struct FilterQuerableStreamingMultiMap<'a, 'b, K:Eq+Hash+Clone+'static,V:Eq+Hash+Clone+'static,
    Source: QuerableStreamingMultiMap<'a, K,V>, Allow: Fn(K, V)->bool + 'a> {
    source: &'b Source,
    listeners:  Rc<MultiSetMessageListeners<'a, (K, V)>>,
    allow: Rc<Allow>
}

impl<'a, 'b, 'c : 'a, K:Eq+Hash+Clone,V:Eq+Hash+Clone, Source: QuerableStreamingMultiMap<'a, K,V>,
        Allow: Fn(K, V)->bool + 'c>
    FilterQuerableStreamingMultiMap<'a, 'b, K,V, Source, Allow> {
    pub fn new(source: &'b Source, allow: Allow)->Self {
        let r = Self {
            allow: Rc::new(allow),
            listeners: Rc::new(MultiSetMessageListeners::new()),
            source};
            
        let lclone=r.listeners.clone();
        let aclone=r.allow.clone();
        source.listeners().listen(move |message| {
            match &message {
                MultiSetModifyMessage::InsertOne((key, value))=> {
                    if (aclone)(key.clone(), value.clone()) {
                        lclone.send(message.clone());
                    }
                },
                MultiSetModifyMessage::RemoveOne((key, value))=>{
                    if (aclone)(key.clone(), value.clone()) {
                        lclone.send(message.clone());
                    }
                }
            }
        }); 
        r
    }
}


// impl<'a, 'b, 'c, K:Eq+Hash+Clone,V:Eq+Hash+Clone, Source: QuerableStreamingMultiMap<'a, K,V> + 'a,
//         Allow: Fn(K, V)->bool + 'a>
//     JoinMultiMap<'a, K,V> for
//      FilterQuerableStreamingMultiMap<'a, 'b, K,V, Source, Allow> {
//      }

impl<'a, 'b, 'c, K:Eq+Hash+Clone,V:Eq+Hash+Clone, Source: QuerableStreamingMultiMap<'a, K,V>,
        Allow: Fn(K, V)->bool + 'a>
    QuerableStreamingMultiMap<'a, K,V> for
     FilterQuerableStreamingMultiMap<'a, 'b, K,V, Source, Allow> {
        fn get(&self, key: &K)->HashSet<V> {
            let source_values=self.source.get(key);
            let mut r=HashSet::new();
            for v in source_values {
                if (self.allow)(key.clone(), v.clone()) {
                    r.insert(v.clone());
                }
            }
            return r;
        }
}

impl <'a, 'b, 'c, K:Eq+Hash+Clone,V:Eq+Hash+Clone, Source: QuerableStreamingMultiMap<'a, K,V>,
        Allow: Fn(K, V)->bool + 'a>
    MessageListenersInterface<'a, MultiSetModifyMessage<(K,V)>> for
     FilterQuerableStreamingMultiMap<'a, 'b, K,V, Source, Allow> {
        fn listeners(&self)->&crate::message_listeners::MessageListeners<'a, MultiSetModifyMessage<(K,V)>> {
            &self.listeners
        }
}


// Order of construction / deconstruction:
// listener should return an Rc<RefMut<Option<Box<&dyn FnMut>>>>???

pub struct JoinQuerableStreamingMultiMap<'a, 'b, 'c, 'd, 'e, K:Eq+Hash+Clone+'static,V:Eq+Hash+Clone+'static, V2:Eq+Hash+Clone+'static,
    Source: QuerableStreamingMultiMap<'c, K,V>, Source2: QuerableStreamingMultiMap<'e, K,V2>> {
    source: RcBorrow<'a, Source>,
    source2: RcBorrow<'b, Source2>,
    listeners:  Rc<MultiSetMessageListeners<'d, (K, (V, V2))>>,
    source_cancel_index: Option<usize>,
    source2_cancel_index: Option<usize>,
    phantom_data: PhantomData<&'c ()>,
    phantom_data2: PhantomData<&'e ()>,
}

impl<'a, 'b, 'c, 'd: 'c+'e, 'e, K:Eq+Hash+Clone,V:Eq+Hash+Clone, V2:Eq+Hash+Clone,
    Source: QuerableStreamingMultiMap<'c, K,V>, Source2: QuerableStreamingMultiMap<'e, K,V2>>
    QuerableStreamingMultiMap<'d, K,(V, V2)> 
    for JoinQuerableStreamingMultiMap<'a, 'b, 'c, 'd, 'e, K,V, V2, Source, Source2> {
        fn get(&self, key: &K)->HashSet<(V, V2)> {
            let source_values=(*self.source).get(key);
            let source_values2=(*self.source2).get(key);
            let mut r=HashSet::new();
            for v in &source_values {
                for v2 in &source_values2 {
                    r.insert((v.clone(), v2.clone()));
                }
            }
            return r;
        }
}

impl <'a, 'b, 'c, 'd: 'c+'e, 'e, K:Eq+Hash+Clone,V:Eq+Hash+Clone, V2:Eq+Hash+Clone,
    Source: QuerableStreamingMultiMap<'c, K,V>, Source2: QuerableStreamingMultiMap<'e, K,V2>>
    MessageListenersInterface<'d, MultiSetModifyMessage<(K,(V, V2))>> 
    for JoinQuerableStreamingMultiMap<'a, 'b, 'c, 'd, 'e, K,V, V2, Source, Source2> {
        fn listeners(&self)->&crate::message_listeners::MessageListeners<'d, MultiSetModifyMessage<(K,(V, V2))>> {
            &*self.listeners
        }
}

impl<'a, 'b, 'c, 'd : 'c+'e, 'e, K:Eq+Hash+Clone,V:Eq+Hash+Clone, V2:Eq+Hash+Clone, 
    Source: QuerableStreamingMultiMap<'c, K,V>+'e,
    Source2: QuerableStreamingMultiMap<'e, K,V2>+'c>
    JoinQuerableStreamingMultiMap<'a, 'b, 'c, 'd, 'e, K,V, V2, Source, Source2> {
    pub fn new(source: &'a Source, source2: &'b Source2)->Self {
        let mut r = Self {
            source: RcBorrow::new(source),
            source2: RcBorrow::new(source2),
            listeners: Rc::new(MultiSetMessageListeners::new()),
            phantom_data: PhantomData,
            phantom_data2: PhantomData,
            source_cancel_index: None,
            source2_cancel_index: None};
        let rlisteners=r.listeners.clone();
        let csource=r.source.get();
        let csource2=r.source2.get();

        r.source_cancel_index=Some(source.listeners().listen(move |message| {
            match message {
                MultiSetModifyMessage::InsertOne((key, value))=> {
                    for value2 in csource2.get(&key) {
                        rlisteners.send(MultiSetModifyMessage::InsertOne((key.clone(), (value.clone(), value2.clone()))));
                    }
                },
                MultiSetModifyMessage::RemoveOne((key, value))=>{
                    for value2 in csource2.get(&key) {
                        rlisteners.send(MultiSetModifyMessage::RemoveOne((key.clone(), (value.clone(), value2.clone()))));
                    }
                }
            }
        }));
        let rlisteners=r.listeners.clone();

        r.source2_cancel_index=Some(source2.listeners().listen(move |message| {
            match message {
                MultiSetModifyMessage::InsertOne((key, value2))=> {
                    for value in csource.get(&key) {
                        rlisteners.send(MultiSetModifyMessage::InsertOne((key.clone(), (value, value2.clone()))));
                    }
                },
                MultiSetModifyMessage::RemoveOne((key, value2))=>{
                    for value in csource.get(&key) {
                        rlisteners.send(MultiSetModifyMessage::RemoveOne((key.clone(), (value, value2.clone()))));
                    }
                }
            }
        }));
        r
    }
}

impl<'a, 'b, 'c, 'd, 'e, K:Eq+Hash+Clone,V:Eq+Hash+Clone, V2:Eq+Hash+Clone, 
    Source: QuerableStreamingMultiMap<'c, K,V>,
    Source2: QuerableStreamingMultiMap<'e, K,V2>>
    Drop for JoinQuerableStreamingMultiMap<'a, 'b, 'c, 'd, 'e, K,V, V2, Source, Source2> {
    fn drop(&mut self) {
        if let Some(index)=self.source_cancel_index {
            self.source.get().listeners().cancel(index);
        }
        if let Some(index)=self.source2_cancel_index {
            self.source2.get().listeners().cancel(index);
        }
    }
}

impl<'a, K:Eq+Hash+Clone+'static,V:Eq+Hash+Clone+'static> MultiSetMessageListeners<'a, (K,V)> {
    pub fn group(&'a self)->Rc<StreamingHashMultiMapWithCount<K,V>> {
        let r=Rc::new(StreamingHashMultiMapWithCount::new());
        let rclone=r.clone();
        self.listeners().listen(move |message| {
                match message {
                    MultiSetModifyMessage::InsertOne((k,v))=>{rclone.insert(k.clone(),v.clone());},
                    MultiSetModifyMessage::RemoveOne((k,v))=>{rclone.remove(k.clone(),v.clone());},
                };
            });
        r
    }

    pub fn reversed(&'a self)->Rc<StreamingHashMultiMapWithCount<V,K>> {
        let r=Rc::new(StreamingHashMultiMapWithCount::new());
        let rclone=r.clone();
        self.listeners().listen(move |message| {
                match message {
                    MultiSetModifyMessage::InsertOne((k,v))=>{rclone.insert(v.clone(),k.clone());},
                    MultiSetModifyMessage::RemoveOne((k,v))=>{rclone.remove(v.clone(),k.clone());},
                };
            });
        r
    }

    pub fn group_by<K2: Eq+Hash+Clone+'static, V2: Eq+Hash+Clone+'static>(&'a self, f:impl Fn(K, V)->(K2, V2) + 'a)->
            Rc<StreamingHashMultiMapWithCount<K2,V2>> {
        let r=Rc::new(StreamingHashMultiMapWithCount::new());
        let rclone=r.clone();
        self.listeners().listen(move |message| {
                match message {
                    MultiSetModifyMessage::InsertOne((k,v))=>{
                        let (k2, v2)=f(k.clone(), v.clone());
                        rclone.insert(k2, v2);
                    },
                    MultiSetModifyMessage::RemoveOne((k,v))=>{
                        let (k2, v2)=f(k.clone(), v.clone());
                        rclone.remove(k2, v2);
                    },
                };
            });
        r
    }
}


#[test]
fn test_get() {
    let map = StreamingHashMultiMapWithCount::new();
    let key = "key";
    let value = "value";
    map.insert(key, value);
    assert_eq!(map.get(&key), HashSet::from_iter(vec![value]));
}

#[test]
fn test_get_one() {
    let map = StreamingHashMultiMapWithCount::new();
    map.insert("key", "value");
    assert_eq!(map.get_one(&"key"), Some("value"));

}

#[test]
fn test_get_one_none() {
    let map: StreamingHashMultiMapWithCount<&str, &str> = StreamingHashMultiMapWithCount::new();
    assert_eq!(map.get_one(&"key"), None);
}

#[test]
fn test_join() {
    let mut map1 = StreamingHashMultiMapWithCount::new();
    let mut map2 = StreamingHashMultiMapWithCount::new();
    // let joined_map = map1.join(&map2);
    let joined_map = JoinQuerableStreamingMultiMap::new(&map1, &map2);
    map1.insert("key", "value");
    map2.insert("key", "value2");
    assert_eq!(joined_map.get_one(&"key"), Some(("value", "value2")));
}

#[test]
fn test_filter() {
    let map = StreamingHashMultiMapWithCount::new();
    map.insert("key", "value");
    map.insert("key2", "value2");
    let filter_map = map.filter_item(|k, _| k=="key");
    assert_eq!(filter_map.get_one(&"key"), Some("value"));
    assert_eq!(filter_map.get_one(&"key2"), None);
}

#[test]
fn test_filter2() {
    let map = StreamingHashMultiMapWithCount::new();
    map.insert("key", "value");
    map.insert("key2", "value2");
    let filter_map = map.filter_item(|k, _| k=="key");
    let filter_map2 = filter_map.filter_item(|k, _| k=="key");
    assert_eq!(filter_map2.get_one(&"key"), Some("value"));
    assert_eq!(filter_map2.get_one(&"key2"), None);
}

#[test]
fn test_filter_join() {
    let mut map1 = StreamingHashMultiMapWithCount::new();
    let mut map2 = StreamingHashMultiMapWithCount::new();
    let filter_map = map1.filter_item(|k, _| k=="key");
    // let joined_map = filter_map.join(&map2);
    let joined_map = JoinQuerableStreamingMultiMap::new(
            &filter_map, &map2);
    map1.insert("key", "value");
    map1.insert("key2", "value3");
    map2.insert("key", "value2");
    map2.insert("key2", "value4");
    assert_eq!(joined_map.get_one(&"key"), Some(("value", "value2")));
    assert_eq!(joined_map.get_one(&"key2"), None);
}
