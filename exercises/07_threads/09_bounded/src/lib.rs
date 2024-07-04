// TODO: Convert the implementation to use bounded channels.
use crate::data::{Ticket, TicketDraft};
use crate::store::{TicketId, TicketStore};
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::mpsc::{sync_channel};

pub mod data;
pub mod store;

#[derive(Clone)]
pub struct TicketStoreClient {
    sender: SyncSender<Command>,
}

impl TicketStoreClient {
    pub fn insert(&self, draft: TicketDraft) -> Result<TicketId, FullError> {
        let (response_sender, response_receiver) = sync_channel(1);
        self.sender.try_send(Command::Insert {
            draft,
            response_channel: response_sender,
        }).map_err(|_| FullError)?;
        Ok(response_receiver.recv().unwrap())
    }

    pub fn get(&self, id: TicketId) -> Result<Option<Ticket>, FullError> {
        let (response_sender, response_receiver) = sync_channel(1);
        self.sender.try_send(Command::Get {
            id,
            response_channel: response_sender,
        }).map_err(|_| FullError)?;
        Ok(response_receiver.recv().unwrap())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("The channel is full")]
pub struct FullError;

pub fn launch(capacity: usize) -> TicketStoreClient {
    let (sender, receiver) = sync_channel(capacity);
    std::thread::spawn(move || server(receiver));
    TicketStoreClient {
        sender
    }
}

enum Command {
    Insert {
        draft: TicketDraft,
        response_channel: SyncSender<TicketId>,
    },
    Get {
        id: TicketId,
        response_channel: SyncSender<Option<Ticket>>,
    },
}

pub fn server(receiver: Receiver<Command>) {
    let mut store = TicketStore::new();
    loop {
        match receiver.recv() {
            Ok(Command::Insert {
                draft,
                response_channel,
            }) => {
                let id = store.add_ticket(draft);
                response_channel.send(id);
            }
            Ok(Command::Get {
                id,
                response_channel,
            }) => {
                let ticket = store.get(id);
                response_channel.send(ticket.cloned());
            }
            Err(_) => {
                // There are no more senders, so we can safely break
                // and shut down the server.
                break;
            }
        }
    }
}
