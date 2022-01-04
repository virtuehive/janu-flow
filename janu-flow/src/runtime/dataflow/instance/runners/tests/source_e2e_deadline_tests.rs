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

use crate::model::deadline::E2EDeadlineRecord;
use crate::model::link::PortDescriptor;
use crate::model::{InputDescriptor, OutputDescriptor};
use crate::runtime::dataflow::instance::link::{LinkReceiver, LinkSender};
use crate::runtime::dataflow::instance::runners::source::SourceRunner;
use crate::runtime::dataflow::instance::runners::NodeRunner;
use crate::runtime::dataflow::loader::{Loader, LoaderConfig};
use crate::runtime::{InstanceContext, RuntimeContext};
use crate::{
    Configuration, Context, Data, Deserializable, DowncastAny, EmptyState, Message, Node, NodeId,
    PortId, Source, State, ZFData, ZFError, ZFResult,
};
use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use std::convert::TryInto;
use std::time::Duration;
use janu::prelude::*;

// ZFUsize implements Data.
#[derive(Debug, Clone)]
pub struct ZFUsize(pub usize);

impl DowncastAny for ZFUsize {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ZFData for ZFUsize {
    fn try_serialize(&self) -> ZFResult<Vec<u8>> {
        Ok(self.0.to_ne_bytes().to_vec())
    }
}

impl Deserializable for ZFUsize {
    fn try_deserialize(bytes: &[u8]) -> ZFResult<Self>
    where
        Self: Sized,
    {
        let value =
            usize::from_ne_bytes(bytes.try_into().map_err(|_| ZFError::DeseralizationError)?);
        Ok(ZFUsize(value))
    }
}

// -------------------------------------------------------------------------------------------------
// Scenarios tested:
//
// 1) the Source is at the "start" of an E2EDeadline, it is propagated.
// -------------------------------------------------------------------------------------------------
struct TestSourceE2EDeadline {}

impl Node for TestSourceE2EDeadline {
    fn initialize(&self, _configuration: &Option<Configuration>) -> ZFResult<State> {
        Ok(State::from::<EmptyState>(EmptyState {}))
    }

    fn finalize(&self, _state: &mut State) -> ZFResult<()> {
        Ok(())
    }
}

#[async_trait]
impl Source for TestSourceE2EDeadline {
    async fn run(&self, _context: &mut Context, _state: &mut State) -> ZFResult<Data> {
        // We sleep for 100ms because we don’t want to push too many messages on the link: the
        // Source will push "1" continuously. No I/O performed so maximum speed…!
        async_std::task::sleep(Duration::from_millis(100)).await;
        Ok(Data::from::<ZFUsize>(ZFUsize(1)))
    }
}

#[test]
fn source_e2e_deadline() {
    let session = janu::open(janu::config::Config::default())
        .wait()
        .unwrap();
    let hlc = Arc::new(uhlc::HLC::default());
    let uuid = uuid::Uuid::new_v4();
    let runtime_context = RuntimeContext {
        session: Arc::new(session),
        hlc,
        loader: Arc::new(Loader::new(LoaderConfig { extensions: vec![] })),
        runtime_name: "runtime--source-e2e-deadline-tests".into(),
        runtime_uuid: uuid,
    };
    let instance_context = InstanceContext {
        flow_id: "flow--source-e2e-deadline-tests".into(),
        instance_id: uuid::Uuid::new_v4(),
        runtime: runtime_context,
    };

    let output: PortId = "OUTPUT".into();
    let (tx_output, rx_output) = flume::unbounded::<Arc<Message>>();
    let receiver_output: LinkReceiver<Message> = LinkReceiver {
        id: output.clone(),
        receiver: rx_output,
    };
    let sender_output: LinkSender<Message> = LinkSender {
        id: output.clone(),
        sender: tx_output,
    };

    let source_id: NodeId = "source".into();
    let source = TestSourceE2EDeadline {};

    let e2e_deadline_1 = E2EDeadlineRecord {
        from: OutputDescriptor {
            node: source_id.clone(),
            output: output.clone(),
        },
        to: InputDescriptor {
            node: "future".into(),
            input: "future-input".into(),
        },
        duration: Duration::from_millis(100),
    };

    let e2e_deadline_2 = E2EDeadlineRecord {
        from: OutputDescriptor {
            node: source_id.clone(),
            output: output.clone(),
        },
        to: InputDescriptor {
            node: "future2".into(),
            input: "future2-input".into(),
        },
        duration: Duration::from_millis(200),
    };

    let source_runner = SourceRunner {
        id: source_id,
        context: instance_context.clone(),
        period: None,
        output: PortDescriptor {
            port_id: output,
            port_type: "ZFUsize".into(),
        },
        links: Arc::new(Mutex::new(vec![sender_output])),
        is_running: Arc::new(Mutex::new(false)),
        state: Arc::new(Mutex::new(source.initialize(&None).unwrap())),
        end_to_end_deadlines: vec![e2e_deadline_1.clone(), e2e_deadline_2.clone()],
        base_resource_name: "test".into(),
        current_recording_resource: Arc::new(Mutex::new(None)),
        is_recording: Arc::new(Mutex::new(false)),
        source: Arc::new(source),
        _library: None,
    };

    let runner = NodeRunner::new(Arc::new(source_runner), instance_context);
    assert!(
        async_std::task::block_on(async_std::future::timeout(Duration::from_secs(5), async {
            let runner_manager = runner.start();

            let (_, message) = receiver_output.recv().await.unwrap();
            if let Message::Data(data_message) = message.as_ref() {
                assert_eq!(data_message.end_to_end_deadlines.len(), 2);
                assert!(data_message
                    .end_to_end_deadlines
                    .iter()
                    .any(|e2e_deadline| *e2e_deadline == e2e_deadline_1));
                assert!(data_message
                    .end_to_end_deadlines
                    .iter()
                    .any(|e2e_deadline| *e2e_deadline == e2e_deadline_2));
            }

            runner_manager.kill().await.unwrap();
            runner_manager.await.unwrap();
        }))
        .is_ok(),
        "Deadlock detected."
    );
}
