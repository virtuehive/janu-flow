//
// Copyright (c) 2017, 2021 Tawedge.
//
// This program and the accompanying materials are made available under the
// terms of the TAW Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   open
//

use janu_flow::async_std::sync::Arc;
use janu_flow::runtime::dataflow::instance::link::{link, LinkReceiver, LinkSender};

async fn same_task_simple() {
    let size = 2;
    let send_id = "0".into();
    let recv_id = "10".into();
    let (sender, receiver) = link::<u8>(Some(size), send_id, recv_id);

    let mut d: u8 = 0;
    // Add the first element
    let res = sender.send(Arc::new(d)).await;
    assert_eq!(res, Ok(()));
    let res = receiver.recv().await;
    assert_eq!(res, Ok(("10".into(), Arc::new(0u8))));

    // Add the second element
    d += 1;
    let res = sender.send(Arc::new(d)).await;
    assert_eq!(res, Ok(()));
    let res = receiver.recv().await;
    assert_eq!(res, Ok(("10".into(), Arc::new(1u8))));
}

async fn recv_task_simple(receiver: LinkReceiver<u8>) {
    let res = receiver.recv().await;
    assert_eq!(res, Ok(("10".into(), Arc::new(0u8))));

    let res = receiver.recv().await;
    assert_eq!(res, Ok(("10".into(), Arc::new(1u8))));

    let res = receiver.recv().await;
    assert_eq!(res, Ok(("10".into(), Arc::new(2u8))));
}

async fn send_task_simple(sender: LinkSender<u8>) {
    let mut d: u8 = 0;
    // Add the first element
    let res = sender.send(Arc::new(d)).await;
    assert_eq!(res, Ok(()));
    // Add the second element
    d += 1;
    let res = sender.send(Arc::new(d)).await;
    assert_eq!(res, Ok(()));

    // Add the 3rd element
    d += 1;
    let res = sender.send(Arc::new(d)).await;
    assert_eq!(res, Ok(()));
}

async fn recv_task_more(receiver: LinkReceiver<u8>) {
    for n in 0u8..255u8 {
        let res = receiver.recv().await;
        assert_eq!(res, Ok(("10".into(), Arc::new(n))));
    }
}

async fn send_task_more(sender: LinkSender<u8>) {
    for n in 0u8..255u8 {
        let res = sender.send(Arc::new(n)).await;
        assert_eq!(res, Ok(()));
    }
}

#[test]
fn ordered_fifo_simple_async() {
    async_std::task::block_on(async move { same_task_simple().await })
}

#[test]
fn ordered_fifo_simple_two_task_async() {
    let size = 2;
    let send_id = "0".into();
    let recv_id = "10".into();
    let (sender, receiver) = link::<u8>(Some(size), send_id, recv_id);

    let h1 = async_std::task::spawn(async move { send_task_simple(sender).await });

    let h2 = async_std::task::spawn(async move { recv_task_simple(receiver).await });

    async_std::task::block_on(async move {
        h1.await;
        h2.await;
    })
}

#[test]
fn ordered_fifo_more_two_task_async() {
    let size = 20;
    let send_id = "0".into();
    let recv_id = "10".into();
    let (sender, receiver) = link::<u8>(Some(size), send_id, recv_id);

    let h1 = async_std::task::spawn(async move { send_task_more(sender).await });

    let h2 = async_std::task::spawn(async move { recv_task_more(receiver).await });

    async_std::task::block_on(async move {
        h1.await;
        h2.await;
    })
}
