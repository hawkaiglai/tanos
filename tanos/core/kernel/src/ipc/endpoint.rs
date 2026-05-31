//! IPC Endpoints
//! Endpoints are communication channels between processes.

use crate::{ProcessId, EndpointId, Rights};
use super::{Message};
use alloc::collections::VecDeque;

pub struct Endpoint {
    pub id: EndpointId,
    pub owner: ProcessId,
    pub rights: Rights,
    pub state: EndpointState,
    pub queue: VecDeque<(ProcessId, Message)>,
}

impl Endpoint {
    pub fn new(id: EndpointId, owner: ProcessId) -> Self {
        Self {
            id,
            owner,
            rights: Rights::all(),
            state: EndpointState::default(),
            queue: VecDeque::new(),
        }
    }

    pub fn with_rights(id: EndpointId, owner: ProcessId, rights: Rights) -> Self {
        Self {
            id,
            owner,
            rights,
            state: EndpointState::default(),
            queue: VecDeque::new(),
        }
    }

    pub fn can_send(&self, _pid: ProcessId) -> bool {
        self.rights.contains(Rights::SEND)
    }

    pub fn can_receive(&self, pid: ProcessId) -> bool {
        self.owner == pid && self.rights.contains(Rights::RECEIVE)
    }

    pub fn enqueue(&mut self, msg: Message) {
        self.queue.push_back((ProcessId::INVALID, msg));
    }

    pub fn enqueue_message(&mut self, sender: ProcessId, msg: Message) {
        self.queue.push_back((sender, msg));
    }

    pub fn dequeue(&mut self) -> Option<Message> {
        self.queue.pop_front().map(|(_, msg)| msg)
    }

    pub fn dequeue_message(&mut self) -> Option<(ProcessId, Message)> {
        self.queue.pop_front()
    }
}

#[derive(Debug, Clone)]
pub enum EndpointState {
    Idle,
    Receiving(ProcessId),
    Call(ProcessId),
}

impl Default for EndpointState {
    fn default() -> Self {
        EndpointState::Idle
    }
}
