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

use async_std::sync::Arc;
use async_trait::async_trait;
use flume::{bounded, Receiver};
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::atomic::{AtomicUsize, Ordering};
use janu_flow::model::link::PortDescriptor;
use janu_flow::model::{InputDescriptor, OutputDescriptor};
use janu_flow::runtime::dataflow::instance::DataflowInstance;
use janu_flow::runtime::dataflow::loader::{Loader, LoaderConfig};
use janu_flow::runtime::RuntimeContext;
use janu_flow::janu_flow_derive::ZFData;
use janu_flow::{
    default_input_rule, default_output_rule, zf_empty_state, Configuration, Context, Data,
    Deserializable, LocalDeadlineMiss, Node, NodeOutput, Operator, PortId, Sink, Source, State,
    ZFData, ZFError, ZFResult,
};

// Data Type

#[derive(Debug, Clone, ZFData)]
pub struct ZFUsize(pub usize);

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

static SOURCE: &str = "Counter";
static DESTINATION: &str = "Counter";

static COUNTER: AtomicUsize = AtomicUsize::new(0);

// SOURCE

struct CountSource {
    rx: Receiver<()>,
}

unsafe impl Send for CountSource {}
unsafe impl Sync for CountSource {}

impl CountSource {
    pub fn new(rx: Receiver<()>) -> Self {
        CountSource { rx }
    }
}

#[async_trait]
impl Source for CountSource {
    async fn run(&self, _context: &mut Context, _state: &mut State) -> janu_flow::ZFResult<Data> {
        let _ = self.rx.recv_async().await;
        COUNTER.fetch_add(1, Ordering::AcqRel);
        let d = ZFUsize(COUNTER.load(Ordering::Relaxed));
        Ok(Data::from::<ZFUsize>(d))
    }
}

impl Node for CountSource {
    fn initialize(&self, _configuration: &Option<Configuration>) -> ZFResult<State> {
        zf_empty_state!()
    }

    fn finalize(&self, _state: &mut State) -> ZFResult<()> {
        Ok(())
    }
}

// SINK

struct ExampleGenericSink;

#[async_trait]
impl Sink for ExampleGenericSink {
    async fn run(
        &self,
        _context: &mut Context,
        _state: &mut State,
        mut input: janu_flow::runtime::message::DataMessage,
    ) -> janu_flow::ZFResult<()> {
        let data = input.get_inner_data().try_get::<ZFUsize>()?;

        assert_eq!(data.0, COUNTER.load(Ordering::Relaxed));

        println!("Example Generic Sink Received: {:?}", input);
        Ok(())
    }
}

impl Node for ExampleGenericSink {
    fn initialize(&self, _configuration: &Option<Configuration>) -> ZFResult<State> {
        zf_empty_state!()
    }

    fn finalize(&self, _state: &mut State) -> ZFResult<()> {
        Ok(())
    }
}

// OPERATOR

#[derive(Debug)]
struct NoOp;

impl Operator for NoOp {
    fn input_rule(
        &self,
        _context: &mut janu_flow::Context,
        state: &mut State,
        tokens: &mut HashMap<PortId, janu_flow::Token>,
    ) -> janu_flow::ZFResult<bool> {
        default_input_rule(state, tokens)
    }

    fn run(
        &self,
        _context: &mut janu_flow::Context,
        _state: &mut State,
        inputs: &mut HashMap<PortId, janu_flow::runtime::message::DataMessage>,
    ) -> janu_flow::ZFResult<HashMap<janu_flow::PortId, Data>> {
        let mut results: HashMap<PortId, Data> = HashMap::new();
        let source: PortId = SOURCE.into();

        let data = inputs
            .get_mut(&source)
            .ok_or_else(|| ZFError::InvalidData("No data".to_string()))?
            .get_inner_data()
            .try_get::<ZFUsize>()?;

        assert_eq!(data.0, COUNTER.load(Ordering::Relaxed));

        results.insert(DESTINATION.into(), Data::from::<ZFUsize>(data.clone()));
        Ok(results)
    }

    fn output_rule(
        &self,
        _context: &mut janu_flow::Context,
        state: &mut State,
        outputs: HashMap<PortId, Data>,
        deadline_miss: Option<LocalDeadlineMiss>,
    ) -> janu_flow::ZFResult<HashMap<janu_flow::PortId, NodeOutput>> {
        assert!(
            deadline_miss.is_none(),
            "Expected `deadline_miss` to be `None`."
        );
        default_output_rule(state, outputs)
    }
}

impl Node for NoOp {
    fn initialize(&self, _configuration: &Option<Configuration>) -> ZFResult<State> {
        zf_empty_state!()
    }

    fn finalize(&self, _state: &mut State) -> ZFResult<()> {
        Ok(())
    }
}

// Run dataflow in single runtime
async fn single_runtime() {
    env_logger::init();

    let (tx, rx) = bounded::<()>(1); // Channel used to trigger source

    let session = Arc::new(janu::open(janu::config::Config::default()).await.unwrap());
    let hlc = async_std::sync::Arc::new(uhlc::HLC::default());
    let rt_uuid = uuid::Uuid::new_v4();
    let ctx = RuntimeContext {
        session,
        hlc,
        loader: Arc::new(Loader::new(LoaderConfig { extensions: vec![] })),
        runtime_name: format!("test-runtime-{}", rt_uuid).into(),
        runtime_uuid: rt_uuid,
    };

    let mut dataflow =
        janu_flow::runtime::dataflow::Dataflow::new(ctx.clone(), "test".into(), None);

    let source = Arc::new(CountSource::new(rx));
    let sink = Arc::new(ExampleGenericSink {});
    let operator = Arc::new(NoOp {});

    dataflow
        .try_add_static_source(
            "counter-source".into(),
            None,
            PortDescriptor {
                port_id: SOURCE.into(),
                port_type: "int".into(),
            },
            source.initialize(&None).unwrap(),
            source,
        )
        .unwrap();

    dataflow
        .try_add_static_sink(
            "generic-sink".into(),
            PortDescriptor {
                port_id: SOURCE.into(),
                port_type: "int".into(),
            },
            sink.initialize(&None).unwrap(),
            sink,
        )
        .unwrap();

    dataflow
        .try_add_static_operator(
            "noop".into(),
            vec![PortDescriptor {
                port_id: SOURCE.into(),
                port_type: "int".into(),
            }],
            vec![PortDescriptor {
                port_id: DESTINATION.into(),
                port_type: "int".into(),
            }],
            None,
            operator.initialize(&None).unwrap(),
            operator,
        )
        .unwrap();

    dataflow
        .try_add_link(
            OutputDescriptor {
                node: "counter-source".into(),
                output: SOURCE.into(),
            },
            InputDescriptor {
                node: "noop".into(),
                input: SOURCE.into(),
            },
            None,
            None,
            None,
        )
        .unwrap();

    dataflow
        .try_add_link(
            OutputDescriptor {
                node: "noop".into(),
                output: DESTINATION.into(),
            },
            InputDescriptor {
                node: "generic-sink".into(),
                input: SOURCE.into(),
            },
            None,
            None,
            None,
        )
        .unwrap();

    let mut instance = DataflowInstance::try_instantiate(dataflow).unwrap();

    let ids = instance.get_nodes();
    for id in &ids {
        instance.start_node(id).await.unwrap();
    }
    tx.send_async(()).await.unwrap();

    janu_flow::async_std::task::sleep(std::time::Duration::from_secs(1)).await;

    for id in &instance.get_sources() {
        instance.stop_node(id).await.unwrap()
    }

    for id in &instance.get_operators() {
        instance.stop_node(id).await.unwrap()
    }

    for id in &instance.get_sinks() {
        instance.stop_node(id).await.unwrap()
    }
}

#[test]
fn run_single_runtime() {
    let h1 = async_std::task::spawn(async move { single_runtime().await });

    async_std::task::block_on(async move { h1.await })
}
