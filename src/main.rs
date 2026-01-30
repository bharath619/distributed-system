use std::{str::FromStr, sync::{ Arc, Mutex}, vec};
use iroh::{Endpoint, protocol::Router};
use tokio::sync::mpsc;
use iroh_tickets::endpoint::EndpointTicket;
use iroh_gossip::{ALPN as GOSSIP_ALPN, TopicId, net::Gossip, api::Event};
use futures::StreamExt;
use serde::{Serialize, Deserialize};
use bytes::Bytes;
use std::io;
use std::io::Write;
use chrono::{self, Utc};
use wincode::{SchemaRead, SchemaWrite};
//key value store

#[derive(Debug, Clone,Serialize, Deserialize,SchemaWrite, SchemaRead)]
struct StoreValue{
    value: String,
    node: String,
    node_id: Option<u16>,
    timestamp: i64, //epoch
}


#[derive(Debug)]
struct KVStore {
    public_key: String,
    node_id: Option<u16>,
    store: Mutex<std::collections::HashMap<String, StoreValue>>
}

#[derive(Debug, Clone,Serialize, Deserialize,SchemaWrite, SchemaRead)]
struct GossipStoreMessage {
    key: String,
    value: StoreValue,
}


impl KVStore {
    fn new(public_key: String, node_id: Option<u16>) -> Self {
        KVStore {
            public_key,
            node_id,
            store: Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn set(&self, key: String, value: String) -> StoreValue {
        let store_value = StoreValue {
            value,
            node: self.public_key.clone(), 
            node_id: self.node_id,
            timestamp: Utc::now().timestamp(),
        };
        let clone = store_value.clone();
        self.store.lock().unwrap().insert(key, store_value);
        return clone;
    }

    fn get(&self, key: &str) -> Option<String> {
        let store = self.store.lock().unwrap();
        if let Some(value) = store.get(key) {
            return Some(value.value.clone());
        }
        return None;
    }

    fn delete(&self, key: &str) {
        self.store.lock().unwrap().remove(key);
    }

    fn merge(&self,key: String, new_value: StoreValue) {
        let mut store = self.store.lock().unwrap();
        if let Some(existing_value) = store.get(&key) {
            // Compare timestamps
            if new_value.timestamp > existing_value.timestamp {
                store.insert(key, new_value);
            }
        } else {
            // Key does not exist, simply insert
            store.insert(key, new_value);
        }
    }

    fn print_store(&self) {
        let store = self.store.lock().unwrap();
        for (key, value) in store.iter() {
            println!("Key: {}, Value: {:?}", key, value);
        }
    }
}



#[tokio::main]
async fn main() -> anyhow::Result<()>  {
    
    let endpoint = Endpoint::builder().bind().await?;
    let kv_store = Arc::new(KVStore::new(
        endpoint.id().to_string(),
        None,
    ));
    let kv_store_clone = kv_store.clone();
    // println!("Endpoint listening on {}", endpoint.id());
    // println!("My endpoint addr: {:?}", endpoint.addr());
    let gossip = Gossip::builder().spawn(endpoint.clone());
    // println!("Value for key1: {:?}", kv_store_2.lock().unwrap());
    let  builder = Router::builder(endpoint.clone());
    let _router =  builder.accept(GOSSIP_ALPN, gossip.clone()).spawn();
    let mut topic_bytes = [0u8; 32];
    topic_bytes[..8].copy_from_slice(b"kv-store");
    let topic = TopicId::from_bytes(topic_bytes);
    // let endpoint = build_transport_addr("475b734def1b57a1afc6eb4cb0a16d918e3fcae58252c32d932705ceb63a8a83", "10.30.250.64", 53023)?;
    // let endpint_id = endpoint.id;
    let mut subscription = gossip.subscribe(topic.clone(),vec![]).await?;
    let (tx, mut rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                // incoming gossip
                event = subscription.next() => {
                    if let Some(Ok(Event::Received(msg))) = event {
                        println!("Received: {:?}", msg.content);
                        let gossip_message: GossipStoreMessage = wincode::deserialize(&msg.content).unwrap();
                        kv_store_clone.merge(gossip_message.key, gossip_message.value);
                    }
                }

                // outgoing broadcast
                Some(msg) = rx.recv() => {
                    let _ = subscription.broadcast(msg).await;
                }
            }
        }
    });
    // tx.send(Bytes::from("hello kv-store"));
    let _ = std::io::stdout().flush();
    let mut input = String::new();
    println!("=== Distributed Key-Value Store === \n\n");
    println!("Enter commands (set/get/delete):");
    println!("Type 'join <ticket>' to join the network or 'ticket' to generate a ticket.");
    loop {
        print!("> ");
        let _ = std::io::stdout().flush();
        io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
        let trimmer_input = input.trim().to_string();
        let parts: Vec<&str> = trimmer_input.split_whitespace().collect();

        match parts.as_slice() {
            ["join", ticket_str] => {
                println!("Usage: join <ticket>");
                let ticket = EndpointTicket::from_str(*ticket_str)?;
                let public_key = ticket.endpoint_addr().id;
                //endpoint.connect(ticket, GOSSIP_ALPN).await?;
                gossip.subscribe(topic.clone(), vec![public_key]).await?;
                println!("Connected to peer");
            }
            ["set", key, value] => {
                let store_value = kv_store.set(key.to_string(), value.to_string());
                let gossip_message = GossipStoreMessage {
                    key: key.to_string(),
                    value: store_value,
                };

                let serialized = wincode::serialize(&gossip_message).unwrap();
                let _ = tx.send(Bytes::from(serialized));
                println!("Set key '{}' to value '{}'", key, value);
            }
            ["get", key] => {   
                let store = kv_store.get(key);
                match store {
                    Some(value) => println!("Value for key '{}': '{}'", key, value),
                    None => println!("Key '{}' not found", key),
                }
            }
            ["delete", key] => {
                kv_store.delete(key);
                let msg = format!("DELETE {}", key);
                let _ = tx.send(Bytes::from(msg));
                println!("Deleted key '{}'", key);
            }
            ["ticket"] => {
                let ticket = EndpointTicket::new(endpoint.addr());
                println!("[Ticket]: {ticket}");
            }
            ["print"] => {
                kv_store.print_store();
            }
            ["q" | "quit" | "exit"] => {
                println!("Exiting...");
                return Ok(());
            }
            [] => {
                // Empty input
            }
            _ => {
                println!("Unknown command");
            }
        }
        input.clear();
    }
    // endpoint.connect(endpoint_addr, GOSSIP_ALPN).await?.accept_bi().await?;
}

    