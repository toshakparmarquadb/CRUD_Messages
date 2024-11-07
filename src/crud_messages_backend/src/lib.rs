// UPDATED CODE
use candid::{CandidType, Principal};
use ic_cdk::api::time;
use ic_cdk::caller;
use ic_cdk_macros::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;

#[derive(CandidType, Serialize, Deserialize, Clone)]
struct Message {
    id: u64,
    author: Principal,
    content: String,
    created_at: u64,
    updated_at: Option<u64>,
    likes: u32,
    replies: Vec<u64>,
    parent_id: Option<u64>,
}

#[derive(CandidType, Serialize, Deserialize)]
struct PaginationParams {
    page: u32,
    limit: u32,
    sort_by: Option<String>,
}

#[derive(CandidType, Serialize, Deserialize)]
struct PaginatedResponse {
    messages: Vec<Message>,
    total: u64,
    page: u32,
    total_pages: u32,
    has_next: bool,
    has_previous: bool,
}

#[derive(CandidType, Serialize, Deserialize, Default)]
struct MessageStats {
    total_messages: u64,
    total_authors: u64,
    messages_today: u64,
}

thread_local! {
    static MESSAGE_STORE: RefCell<BTreeMap<u64, Message>> = RefCell::new(BTreeMap::new());
    static NEXT_ID: RefCell<u64> = RefCell::new(1);
    static AUTHOR_MESSAGE_COUNT: RefCell<BTreeMap<Principal, u32>> = RefCell::new(BTreeMap::new());
}

#[init]
fn init() {
    NEXT_ID.with(|counter| *counter.borrow_mut() = 1);
}

// CREATE
#[update]
fn create_message(content: String, parent_id: Option<u64>) -> Result<Message, String> {
    if content.trim().is_empty() {
        return Err("Message content cannot be empty".to_string());
    }

    if let Some(parent) = parent_id {
        if !MESSAGE_STORE.with(|store| store.borrow().contains_key(&parent)) {
            return Err("Parent message not found".to_string());
        }
    }

    let caller = caller();
    let id = NEXT_ID.with(|counter| {
        let current = *counter.borrow();
        *counter.borrow_mut() = current + 1;
        current
    });

    let message = Message {
        id,
        author: caller,
        content,
        created_at: time(),
        updated_at: None,
        likes: 0,
        replies: Vec::new(),
        parent_id,
    };

    if let Some(parent_id) = parent_id {
        MESSAGE_STORE.with(|store| {
            let mut store = store.borrow_mut();
            if let Some(parent_message) = store.get_mut(&parent_id) {
                parent_message.replies.push(id);
            }
        });
    }

    AUTHOR_MESSAGE_COUNT.with(|count| {
        let mut count = count.borrow_mut();
        *count.entry(caller).or_insert(0) += 1;
    });

    MESSAGE_STORE.with(|store| {
        store.borrow_mut().insert(id, message.clone());
    });

    Ok(message)
}

// READ
#[query]
fn get_message(id: u64) -> Result<Message, String> {
    MESSAGE_STORE.with(|store| {
        store
            .borrow()
            .get(&id)
            .cloned()
            .ok_or_else(|| "Message not found".to_string())
    })
}

#[query]
fn get_messages(params: PaginationParams) -> PaginatedResponse {
    MESSAGE_STORE.with(|store| {
        let store = store.borrow();
        let total = store.len() as u64;
        let total_pages = ((total as f64) / (params.limit as f64)).ceil() as u32;
        
        let mut messages: Vec<Message> = store
            .values()
            .filter(|msg| msg.parent_id.is_none())
            .cloned()
            .collect();

        match params.sort_by.as_deref() {
            Some("oldest") => messages.sort_by_key(|m| m.created_at),
            Some("popular") => messages.sort_by_key(|m| std::cmp::Reverse(m.likes)),
            _ => messages.sort_by_key(|m| std::cmp::Reverse(m.created_at)),
        }

        let skip = (params.page - 1) * params.limit;
        let messages = messages
            .into_iter()
            .skip(skip as usize)
            .take(params.limit as usize)
            .collect();

        PaginatedResponse {
            messages,
            total,
            page: params.page,
            total_pages,
            has_next: params.page < total_pages,
            has_previous: params.page > 1,
        }
    })
}

// UPDATE
#[update]
fn update_message(id: u64, new_content: String) -> Result<Message, String> {
    if new_content.trim().is_empty() {
        return Err("Message content cannot be empty".to_string());
    }

    MESSAGE_STORE.with(|store| {
        let mut store = store.borrow_mut();
        let message = store.get_mut(&id).ok_or("Message not found")?;
        
        // Only the author can update their message
        if message.author != caller() {
            return Err("Only the author can update this message".to_string());
        }

        message.content = new_content;
        message.updated_at = Some(time());
        
        Ok(message.clone())
    })
}

// DELETE
#[update]
fn delete_message(id: u64) -> Result<(), String> {
    MESSAGE_STORE.with(|store| {
        let mut store = store.borrow_mut();
        
        // First, check if the message exists and verify ownership
        let message = store.get(&id).ok_or("Message not found")?;
        if message.author != caller() {
            return Err("Only the author can delete this message".to_string());
        }

        // Clone the necessary data before removing the message
        let parent_id = message.parent_id;
        let author = message.author;

        // If this message has a parent, remove it from parent's replies
        if let Some(parent_id) = parent_id {
            if let Some(parent) = store.get_mut(&parent_id) {
                parent.replies.retain(|&reply_id| reply_id != id);
            }
        }

        // Remove the message
        store.remove(&id);

        // Update author's message count in a separate operation
        AUTHOR_MESSAGE_COUNT.with(|count| {
            let mut count = count.borrow_mut();
            if let Some(author_count) = count.get_mut(&author) {
                *author_count = author_count.saturating_sub(1);
            }
        });

        Ok(())
    })
}

// Additional helper functions
#[update]
fn like_message(id: u64) -> Result<(), String> {
    MESSAGE_STORE.with(|store| {
        let mut store = store.borrow_mut();
        if let Some(message) = store.get_mut(&id) {
            message.likes += 1;
            Ok(())
        } else {
            Err("Message not found".to_string())
        }
    })
}

#[query]
fn get_message_thread(id: u64) -> Result<Vec<Message>, String> {
    MESSAGE_STORE.with(|store| {
        let store = store.borrow();
        let message = store.get(&id).ok_or("Message not found")?;
        
        let mut thread = vec![message.clone()];
        for reply_id in &message.replies {
            if let Some(reply) = store.get(reply_id) {
                thread.push(reply.clone());
            }
        }
        Ok(thread)
    })
}

#[query]
fn get_stats() -> MessageStats {
    let current_time = time();
    let day_in_nanos = 24 * 60 * 60 * 1_000_000_000;

    MESSAGE_STORE.with(|store| {
        let store = store.borrow();
        let mut authors = std::collections::HashSet::new();
        let mut messages_today = 0;

        for message in store.values() {
            authors.insert(message.author);
            if current_time - message.created_at < day_in_nanos {
                messages_today += 1;
            }
        }

        MessageStats {
            total_messages: store.len() as u64,
            total_authors: authors.len() as u64,
            messages_today,
        }
    })
}

ic_cdk::export_candid!();
