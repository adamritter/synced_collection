use std::{collections::{HashSet, HashMap}, cell::RefCell, rc::{Rc, Weak}, marker::PhantomData};

use crate::{multi_set::{MultiSetModifyMessage, MultiSetMessageListeners}, message_listeners::{MessageListenersInterface, MessageListeners}, rc_borrow::{RcBorrow, Borrow}};
use std::hash::Hash;

pub trait QuerableStreamingMultiMapGetter<K:Eq+Hash+Clone + 'static,V:Eq+Hash+Clone+'static> {
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
pub trait QuerableStreamingMultiMap<'source, 'listener, K:Eq+Hash+Clone + 'static,V:Eq+Hash+Clone+'static> :
     MessageListenersInterface<'listener, MultiSetModifyMessage<(K,V)>>  {
    type Getter : QuerableStreamingMultiMapGetter<K,V> + 'source + 'listener;
    fn getter(&self)->&Self::Getter;
    fn get(&self, key: &K)->HashSet<V> {
        self.getter().get(key)
    }
    fn get_one(&self, key: &K)->Option<V> {
        self.getter().get_one(key)
    }
    fn filter_item<'last_source, Allow: Fn(K,V)->bool>(&'last_source self, allow: Allow)->
            FilterQuerableStreamingMultiMap<'source, 'listener, 'last_source, K, V, Self, Allow> {
        FilterQuerableStreamingMultiMap::new(self, allow)
    }
    fn filter_item_old<Allow: Fn(K,V)->bool+'listener>(&'listener self, allow: Allow)->
        Rc<MessageListeners<'listener, MultiSetModifyMessage<(K,V)>>> {
        let f = move |m: &MultiSetModifyMessage<(K,V)>| {
            match m {
                MultiSetModifyMessage::InsertOne((k,v)) => allow(k.clone(), v.clone()),
                MultiSetModifyMessage::RemoveOne((k,v)) => allow(k.clone(), v.clone())
            }
        };
        self.filter(f)
    }
    fn map<T2:Clone+'static>(&'source self, f: impl Fn(K, V)->T2+'listener)->
            Rc<MultiSetMessageListeners<'listener, T2>> {
        self.listeners().map_items(move |(k, v)| f(k, v))
    }
    fn join<'last_source,V2:Eq+Hash+Clone+'static, Source2: QuerableStreamingMultiMap<'source, 'listener, K, V2>>(
            &'last_source self, other: &'last_source Source2)->
            JoinQuerableStreamingMultiMap<'source, 'listener, 'last_source, K, V, V2, Self, Source2> {
        JoinQuerableStreamingMultiMap::new(self, other)
    }
    
    fn reversed<'last_source>(&'last_source self)->Rc<StreamingHashMultiMapWithCount<'listener, V, K>> {
        self.listeners().reversed()
    }
    fn group_by<'last_source, K2: Eq+Hash+Clone+'static, V2: Eq+Hash+Clone+'static>(
            &'last_source self, f:impl Fn(K, V)->(K2, V2) + 'listener) ->
            Rc<StreamingHashMultiMapWithCount<'listener, K2,V2>> {
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
pub struct StreamingHashMultiMapWithCount<'listener, K: Eq+Hash+Clone+'static, V: Eq+Hash+Clone+'static> {
    listeners: MultiSetMessageListeners<'listener, (K, V)>,
    data: RefCell<HashMap<K, HashMap<V, u64>>>
}

impl<'a, K: Eq+Hash+Clone+'static, V: Eq+Hash+Clone+'static> StreamingHashMultiMapWithCount<'a, K, V> {
    pub fn new()->Self {
        Self {listeners: MultiSetMessageListeners::new(), data: RefCell::new(HashMap::new())}
    }
}

impl<K: Eq+Hash+Clone + 'static, V: Eq+Hash+Clone+'static> 
    QuerableStreamingMultiMapGetter<K,V> for RefCell<HashMap<K, HashMap<V, u64>>> {
    fn get(&self, key: &K)->HashSet<V> {
        let data = self.borrow();
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
}

impl<'listener, K: Eq+Hash+Clone + 'static, V: Eq+Hash+Clone+'static>
     QuerableStreamingMultiMap<'_, 'listener, K,V>
     for StreamingHashMultiMapWithCount<'listener, K, V> {
    type Getter = RefCell<HashMap<K, HashMap<V, u64>>>;
    fn getter(&self)->&Self::Getter {
        &self.data
    }
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
 
pub struct FilterQuerableStreamingMultiMap<'source, 'listener, 'last_source, K:Eq+Hash+Clone+'static,V:Eq+Hash+Clone+'static,
    Source: QuerableStreamingMultiMap<'source, 'listener, K,V>,
    Allow: Fn(K, V)->bool + 'source + 'listener> {
    source: &'last_source Source,
    listeners:  Rc<MultiSetMessageListeners<'listener, (K, V)>>,
    allow: Rc<Allow>,
    getter: FilterQuerableStreamingMultiMapGetter<K,V, Source::Getter, Allow>,
    source_getter: RcBorrow<'last_source, Source::Getter>
}

pub struct FilterQuerableStreamingMultiMapGetter<K:Eq+Hash+Clone+'static,V:Eq+Hash+Clone+'static,
        SourceGetter: QuerableStreamingMultiMapGetter<K,V>,
        Allow: Fn(K, V)->bool> {
    source: Rc<Borrow<SourceGetter>>,
    allow: Rc<Allow>,
    phantom_data: PhantomData<(K,V)>
}

impl <'source, 'listener, K:Eq+Hash+Clone,V:Eq+Hash+Clone,
        Getter: QuerableStreamingMultiMapGetter<K,V>,
        Allow: Fn(K, V)->bool + 'source + 'listener>
    QuerableStreamingMultiMapGetter<K,V>
            for FilterQuerableStreamingMultiMapGetter<K,V, Getter, Allow> {
        fn get(&self, key: &K)->HashSet<V> {
        let values=self.source.get(key);
        let allow=&self.allow;
        values.into_iter().filter(|v| allow(key.clone(), v.clone())).collect()
    }
}

impl<'source, 'listener, 'last_source, K:Eq+Hash+Clone,V:Eq+Hash+Clone,
        Source: QuerableStreamingMultiMap<'source, 'listener, K,V>,
        Allow: Fn(K, V)->bool + 'source + 'listener>
    FilterQuerableStreamingMultiMap<'source, 'listener, 'last_source, K,V, Source, Allow> {
    pub fn new(source: &'last_source Source, allow: Allow)->Self {
        let source_getter=RcBorrow::new(source.getter());
        let allow=Rc::new(allow);
        let r = Self {
            allow: allow.clone(),
            listeners: Rc::new(MultiSetMessageListeners::new()),
            source,
            getter: FilterQuerableStreamingMultiMapGetter {
                source: source_getter.get(),
                allow,
                phantom_data: PhantomData
            },
            source_getter
        };
            
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

impl<'source, 'listener, 'last_source, K:Eq+Hash+Clone,V:Eq+Hash+Clone,
        Source: QuerableStreamingMultiMap<'source, 'listener, K,V>,
        Allow: Fn(K, V)->bool + 'source + 'listener>
    QuerableStreamingMultiMap<'source, 'listener, K,V> for
     FilterQuerableStreamingMultiMap<'source, 'listener, 'last_source, K,V, Source, Allow> {
        type Getter=FilterQuerableStreamingMultiMapGetter<K,V, Source::Getter, Allow>;
        fn getter(&self)->&Self::Getter {
            &self.getter
        }
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

impl <'source, 'listener, 'last_source, K:Eq+Hash+Clone,V:Eq+Hash+Clone,
        Source: QuerableStreamingMultiMap<'source, 'listener, K,V>,
        Allow: Fn(K, V)->bool + 'source + 'listener>
    MessageListenersInterface<'listener, MultiSetModifyMessage<(K,V)>> for
     FilterQuerableStreamingMultiMap<'source, 'listener, 'last_source, K,V, Source, Allow> {
        fn listeners(&self)->&MessageListeners<'listener, MultiSetModifyMessage<(K,V)>> {
            &self.listeners
        }
}


// Order of construction / deconstruction:
// listener should return an Rc<RefMut<Option<Box<&dyn FnMut>>>>???

pub struct JoinQuerableStreamingMultiMap<'source, 'listener, 'last_source, K:Eq+Hash+Clone+'static,V:Eq+Hash+Clone+'static, V2:Eq+Hash+Clone+'static,
    Source: QuerableStreamingMultiMap<'source, 'listener, K,V>,
    Source2: QuerableStreamingMultiMap<'source, 'listener, K,V2>> {
    source: RcBorrow<'last_source, Source>,
    source2: RcBorrow<'last_source, Source2>,
    listeners:  Rc<MultiSetMessageListeners<'listener, (K, (V, V2))>>,
    source_cancel_index: Option<usize>,
    source2_cancel_index: Option<usize>,
    getter: JoinQuerableStreamingMultiMapGetter<K,V,V2, Source::Getter, Source2::Getter>,
    source_getter: RcBorrow<'last_source, Source::Getter>,
    source2_getter: RcBorrow<'last_source, Source2::Getter>,
}


pub struct JoinQuerableStreamingMultiMapGetter<K:Eq+Hash+Clone+'static
    ,V:Eq+Hash+Clone+'static, V2:Eq+Hash+Clone+'static,
    SourceGetter: QuerableStreamingMultiMapGetter<K,V>,
    SourceGetter2: QuerableStreamingMultiMapGetter<K,V2>> {
    source: Rc<Borrow<SourceGetter>>,
    source2: Rc<Borrow<SourceGetter2>>,
    phantom_data: PhantomData<(K,V,V2)>

}

impl <'source, K:Eq+Hash+Clone,V:Eq+Hash+Clone, V2:Eq+Hash+Clone,
    Getter: QuerableStreamingMultiMapGetter<K,V>,
    Getter2: QuerableStreamingMultiMapGetter<K,V2>>
    QuerableStreamingMultiMapGetter<K,(V, V2)> 
    for JoinQuerableStreamingMultiMapGetter<K,V, V2, Getter, Getter2> {
        fn get(&self, key: &K)->HashSet<(V, V2)> {
            let source_values=self.source.get(key);
            let source_values2=self.source2.get(key);
            let mut r=HashSet::new();
            for v in &source_values {
                for v2 in &source_values2 {
                    r.insert((v.clone(), v2.clone()));
                }
            }
            return r;
        }
}

impl<'source, 'listener, 'last_source, K:Eq+Hash+Clone,V:Eq+Hash+Clone, V2:Eq+Hash+Clone,
    Source: QuerableStreamingMultiMap<'source, 'listener, K,V>,
    Source2: QuerableStreamingMultiMap<'source, 'listener, K,V2>>
    QuerableStreamingMultiMap<'source, 'listener, K,(V, V2)> 
    for JoinQuerableStreamingMultiMap<'source, 'listener, 'last_source, K,V, V2, Source, Source2> {
        type Getter = JoinQuerableStreamingMultiMapGetter<K,V, V2, Source::Getter, Source2::Getter>;
        fn getter(&self)->&Self::Getter {
            &self.getter
        }
}

impl <'source, 'listener, 'last_source, K:Eq+Hash+Clone,V:Eq+Hash+Clone, V2:Eq+Hash+Clone,
    Source: QuerableStreamingMultiMap<'source, 'listener, K,V>,
    Source2: QuerableStreamingMultiMap<'source, 'listener, K,V2>>
    MessageListenersInterface<'listener, MultiSetModifyMessage<(K,(V, V2))>> 
    for JoinQuerableStreamingMultiMap<'source, 'listener, 'last_source, K,V, V2, Source, Source2> {
        fn listeners(&self)->&MessageListeners<'listener, MultiSetModifyMessage<(K,(V, V2))>> {
            &*self.listeners
        }
}

impl<'source, 'listener, 'last_source, K:Eq+Hash+Clone,V:Eq+Hash+Clone, V2:Eq+Hash+Clone, 
    Source: QuerableStreamingMultiMap<'source, 'listener, K,V>,
    Source2: QuerableStreamingMultiMap<'source, 'listener, K,V2>>
    JoinQuerableStreamingMultiMap<'source, 'listener, 'last_source, K,V, V2, Source, Source2> {
    pub fn new(source: &'last_source Source, source2: &'last_source Source2)->Self {
        let source_getter=RcBorrow::new(source.getter());
        let source2_getter=RcBorrow::new(source2.getter());
        let mut r = Self {
            source: RcBorrow::new(source),
            source2: RcBorrow::new(source2),
            listeners: Rc::new(MultiSetMessageListeners::new()),
            source_cancel_index: None,
            source2_cancel_index: None,
            
            getter: JoinQuerableStreamingMultiMapGetter {
                source: source_getter.get(),
                source2: source2_getter.get(),
                phantom_data: PhantomData
            },
            source_getter,
            source2_getter};
        let rlisteners=r.listeners.clone();
        let csource=r.getter.source.clone();
        let csource2=r.getter.source2.clone();

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

impl<'source, 'listener, 'last_source, K:Eq+Hash+Clone,V:Eq+Hash+Clone, V2:Eq+Hash+Clone, 
    Source: QuerableStreamingMultiMap<'source, 'listener, K,V>,
    Source2: QuerableStreamingMultiMap<'source, 'listener, K,V2>>
    Drop for JoinQuerableStreamingMultiMap<'source, 'listener, 'last_source, K,V, V2, Source, Source2> {
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

    pub fn reversed<'last_source>(&'last_source self)->
            Rc<StreamingHashMultiMapWithCount<'a, V,K>> {
        self.group_by(|k,v| (v,k))
    }

    pub fn group_by<'last_source, K2: Eq+Hash+Clone+'static, V2: Eq+Hash+Clone+'static>(
            &'last_source self, f:impl Fn(K, V)->(K2, V2) + 'a)->
            Rc<StreamingHashMultiMapWithCount<'a, K2,V2>> {
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
    let joined_map = map1.join(&map2);
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
    let joined_map = filter_map.join(&map2);
    map1.insert("key", "value");
    map1.insert("key2", "value3");
    map2.insert("key", "value2");
    map2.insert("key2", "value4");
    assert_eq!(joined_map.get_one(&"key"), Some(("value", "value2")));
    assert_eq!(joined_map.get_one(&"key2"), None);
}

#[test]
fn test_group_by() {
    let mut map1 = StreamingHashMultiMapWithCount::new();
    let group_map = map1.group_by(|k, v| (v, k));
    map1.insert("key", "value");
    map1.insert("key", "value2");
    map1.insert("key2", "value3");
    assert_eq!(group_map.get_one(&"value"), Some("key"));
}

#[test]
fn test_reversed() {
    let mut map1 = StreamingHashMultiMapWithCount::new();
    map1.insert("key", "value");
    map1.insert("key", "value2");
    map1.insert("key2", "value3");
    // let group_map = map1.reversed();
    let l=map1.listeners();
    // let group_map = l.reversed();
    // assert_eq!(group_map.get_one(&"value"), Some("key"));
}