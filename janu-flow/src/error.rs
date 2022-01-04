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

use crate::serde::{Deserialize, Serialize};
use crate::{NodeId, PortId, PortType};
use std::convert::From;
use uuid::Uuid;
use jrpc::jrpcresult::JRPCError;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum ZFError {
    GenericError,
    SerializationError,
    DeseralizationError,
    MissingState,
    InvalidState,
    Unimplemented,
    Unsupported,
    Empty,
    MissingConfiguration,
    VersionMismatch,
    Disconnected,
    Uncompleted(String),
    RecvError(String),
    SendError(String),
    MissingInput(String),
    MissingOutput(String),
    InvalidData(String),
    IOError(String),
    JanuError(String),
    LoadingError(String),
    ParsingError(String),
    #[serde(skip_serializing, skip_deserializing)]
    RunnerStopError(crate::async_std::channel::RecvError),
    #[serde(skip_serializing, skip_deserializing)]
    RunnerStopSendError(crate::async_std::channel::SendError<()>),
    InstanceNotFound(Uuid),
    RPCError(JRPCError),
    SourceDoNotHaveInputs,
    ReceiverDoNotHaveInputs,
    SinkDoNotHaveOutputs,
    SenderDoNotHaveOutputs,
    // Validation Error
    DuplicatedNodeId(NodeId),
    DuplicatedPort((NodeId, PortId)),
    DuplicatedLink(((NodeId, PortId), (NodeId, PortId))),
    MultipleOutputsToInput((NodeId, PortId)),
    PortTypeNotMatching((PortType, PortType)),
    NodeNotFound(NodeId),
    PortNotFound((NodeId, PortId)),
    PortNotConnected((NodeId, PortId)),
    NotRecoding,
    AlreadyRecording,
    NoPathBetweenNodes(((NodeId, PortId), (NodeId, PortId))),
}

impl From<JRPCError> for ZFError {
    fn from(err: JRPCError) -> Self {
        Self::RPCError(err)
    }
}

impl From<flume::RecvError> for ZFError {
    fn from(err: flume::RecvError) -> Self {
        Self::RecvError(format!("{:?}", err))
    }
}

impl From<crate::async_std::channel::RecvError> for ZFError {
    fn from(err: async_std::channel::RecvError) -> Self {
        Self::RunnerStopError(err)
    }
}

impl From<crate::async_std::channel::SendError<()>> for ZFError {
    fn from(err: async_std::channel::SendError<()>) -> Self {
        Self::RunnerStopSendError(err)
    }
}

impl From<flume::TryRecvError> for ZFError {
    fn from(err: flume::TryRecvError) -> Self {
        match err {
            flume::TryRecvError::Disconnected => Self::Disconnected,
            flume::TryRecvError::Empty => Self::Empty,
        }
    }
}

impl<T> From<flume::SendError<T>> for ZFError {
    fn from(err: flume::SendError<T>) -> Self {
        Self::SendError(format!("{:?}", err))
    }
}

impl From<std::io::Error> for ZFError {
    fn from(err: std::io::Error) -> Self {
        Self::IOError(format!("{}", err))
    }
}

impl From<janu_util::core::Error> for ZFError {
    fn from(err: janu_util::core::Error) -> Self {
        Self::JanuError(format!("{}", err))
    }
}

impl From<libloading::Error> for ZFError {
    fn from(err: libloading::Error) -> Self {
        Self::LoadingError(format!("Error when loading the library: {}", err))
    }
}

#[cfg(feature = "data_json")]
impl From<serde_json::Error> for ZFError {
    fn from(_err: serde_json::Error) -> Self {
        Self::SerializationError
    }
}

#[cfg(feature = "data_json")]
impl From<std::str::Utf8Error> for ZFError {
    fn from(_err: std::str::Utf8Error) -> Self {
        Self::SerializationError
    }
}

impl std::fmt::Display for ZFError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
