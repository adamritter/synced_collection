use chrono::{NaiveDateTime,Utc};
use uuid::Uuid;
use std::fmt::Debug;
use crate::{StreamingHashMultiMapWithCount, QuerableStreamingMultiMap, message_listeners::{MessageListenersInterface}, MultiSetModifyMessage};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ClientId {

}

/// A function decorator that puts its all arguments into a message and sends it to a message listener.

macro_rules! add_to_web_socket {
    ($ws:expr, $($arg:expr),*) => {
        {
            $(
                $arg.add_to_web_socket_by_client($ws);
            )*
        }
    };
}

pub struct WebSocketByClient {
}
impl WebSocketByClient {
    pub fn new()->Self {
        Self {}
    }
    pub fn client_collection<M: Debug+Clone+'static>(&self, ml: &impl MessageListenersInterface<M>) {
        ml.listen(|m| {
            println!("client_collection: {:?}", m);
        });
    }
    pub fn client_fn<T>(&self, _f: impl FnMut(ClientId, T)) {
        unimplemented!()
    }
}

trait FnAdd<T> {
    fn add_to_web_socket_by_client(&mut self, w: &WebSocketByClient);
}

impl<T, F:FnMut(ClientId, T)> FnAdd<T> for F {
    fn add_to_web_socket_by_client(&mut self, w: &WebSocketByClient) {
        w.client_fn(self);
    }
}

trait MessageListenersInterfaceAdd<M> {
    fn add_to_web_socket_by_client(&self, w: &WebSocketByClient);
}

impl<M: Debug+Clone+'static, I: MessageListenersInterface<M>>  MessageListenersInterfaceAdd<M>  for I {
    fn add_to_web_socket_by_client(&self, w: &WebSocketByClient) {
        w.client_collection(self);
    }
}

// A setmodify message source contains a set and has a has(&key) function. It guarantees to have the output
// at most once.
// Operations without extra memory usage:
// - Joining 2 sets creates a setmodifymessagesource.
// - Joining a setmodifymessagesource with a hashmap key to create (key, value) setmodifymessagesource
// - Joining 2 hashmaps to create a (key, value, value) setmodifymessagesource
// - Map a hashmultimap over an optional value message source can make it possible to
//    notify the minimal amount of clients on a change in a hashmap
// - The most important traits are QuerableStreamingMultiMap (that can be joined into one on the same key)
//     and StreamingMultiSet (that can't be joined, but can be mapped and grouped (for a Key,Value tuple))

pub fn test_twitter() {
    let tweets: StreamingHashMultiMapWithCount<Uuid, (NaiveDateTime, String)>=StreamingHashMultiMapWithCount::new();
    let follows: StreamingHashMultiMapWithCount<Uuid, Uuid>=StreamingHashMultiMapWithCount::new();
    // uid_by_client is used as a map instead of multimap.
    let uid_by_client : StreamingHashMultiMapWithCount<ClientId, Uuid> = StreamingHashMultiMapWithCount::new();
    let clients_by_uid= uid_by_client.reversed();
    let clients_and_follows_by_uid =
            clients_by_uid.join(&follows);
    let followed_by_client= clients_and_follows_by_uid.group_by(
            |_logged_in_user_id, (client_id, followed_user_id)|
            (client_id, followed_user_id));
    let followed = clients_and_follows_by_uid.group_by(
        |logged_in_user_id, (client_id, followed_user_id)|
        (followed_user_id, (client_id, logged_in_user_id))
        );
    let seen_tweets = followed.join(&tweets);
    let seen_by_client = seen_tweets.group_by(
            |followed_user_id,
             ((client_id, logged_in_user_id),
                (time, string))|
            (client_id, (logged_in_user_id, followed_user_id, time, string)));
    let mut ws = WebSocketByClient::new();
    let follow=|client_id, uuid| {
        let user_id = uid_by_client.get_one(&client_id);
        if let Some(user_id) = user_id {
            follows.insert(user_id, uuid);
        }
    };
    let set_uuid= |client_id, uuid|
        uid_by_client.set(client_id, uuid);
    let create_tweet = |client_id, s| {
        let user_id=uid_by_client.get_one(&client_id);
        if let Some(user_id)=user_id {
            tweets.insert(user_id, (Utc::now().naive_utc(), s));
        }
    };
    renderer(&mut ws, &seen_by_client, followed_by_client, follow, set_uuid, create_tweet);
}


fn renderer(ws: &mut WebSocketByClient,
    seen_tweets: &impl MessageListenersInterface<MultiSetModifyMessage<(ClientId, (Uuid, Uuid, NaiveDateTime, String))>>,
    followed_by_client: impl MessageListenersInterface<MultiSetModifyMessage<(ClientId, Uuid)>>,
    mut follow: impl FnMut(ClientId, Uuid),
    mut set_uuid: impl FnMut(ClientId, Uuid),
    mut create_tweet: impl FnMut(ClientId, String)) {
    add_to_web_socket!(ws, seen_tweets, followed_by_client, follow, set_uuid, create_tweet);

}
