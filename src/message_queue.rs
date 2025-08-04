// Copyright (C) 2025  Rafael Carvalho <contact@rafaelrc.com>

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as published by
// the Free Software Foundation.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

// SPDX-License-Identifier: GPL-3.0-only

use std::{
    error::Error,
    sync::{Arc, mpsc},
};

use nix::sys::{
    epoll::{Epoll, EpollEvent, EpollFlags},
    eventfd::{self, EfdFlags, EventFd},
};

#[derive(Clone)]
pub struct MessageQueueSender<T> {
    sender: mpsc::Sender<T>,
    eventfd: Arc<eventfd::EventFd>,
}

pub struct MessageQueueReceiver<T> {
    receiver: mpsc::Receiver<T>,
    eventfd: Arc<eventfd::EventFd>,
}

pub fn message_queue<T: Clone>(
    epoll: &Epoll,
    queue_id: u64,
) -> Result<(MessageQueueSender<T>, MessageQueueReceiver<T>), Box<dyn Error>> {
    let (sender, receiver) = mpsc::channel::<T>();

    let eventfd = EventFd::from_flags(EfdFlags::EFD_SEMAPHORE)?;
    epoll.add(&eventfd, EpollEvent::new(EpollFlags::EPOLLIN, queue_id))?;

    let eventfd = Arc::new(eventfd);

    let message_queue_sender = MessageQueueSender {
        sender,
        eventfd: eventfd.clone(),
    };

    let message_queue_receiver = MessageQueueReceiver {
        receiver,
        eventfd: eventfd.clone(),
    };
    log::debug!(target: "MessageQueue::new", "Created new message queue with ID {queue_id}");
    Ok((message_queue_sender, message_queue_receiver))
}

impl<'a, T: 'a + Clone> MessageQueueSender<T> {
    pub fn send(&self, payload: T) -> Result<(), Box<dyn Error + 'a>> {
        self.sender.send(payload)?;
        self.eventfd.write(1)?;
        Ok(())
    }
}

impl<T: Clone> MessageQueueReceiver<T> {
    pub fn recv(&self) -> Result<T, Box<dyn Error>> {
        self.eventfd.read()?;
        Ok(self.receiver.recv()?)
    }
}
