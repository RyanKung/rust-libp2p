// Copyright 2018 Parity Technologies (UK) Ltd.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use futures::prelude::*;
use libp2p_core::nodes::handled_node::NodeHandlerEndpoint;
use libp2p_core::nodes::protocols_handler::{ProtocolsHandler, ProtocolsHandlerEvent};
use libp2p_core::ConnectionUpgrade;
use smallvec::SmallVec;
use std::io;
use tokio_io::{AsyncRead, AsyncWrite};
use void::Void;
use {IdentifySender, IdentifyOutput, IdentifyProtocolConfig};

/// Protocol handler that identifies the remote at a regular period.
pub struct IdentifyListenHandler<TSubstream> {
    /// Configuration for the protocol.
    config: IdentifyProtocolConfig,

    /// List of senders to yield to the user.
    pending_result: SmallVec<[IdentifySender<TSubstream>; 4]>,

    /// True if `shutdown` has been called.
    shutdown: bool,
}

impl<TSubstream> IdentifyListenHandler<TSubstream> {
    /// Builds a new `IdentifyListenHandler`.
    #[inline]
    pub fn new() -> Self {
        IdentifyListenHandler {
            config: IdentifyProtocolConfig,
            pending_result: SmallVec::new(),
            shutdown: false,
        }
    }
}

impl<TSubstream> ProtocolsHandler for IdentifyListenHandler<TSubstream>
where
    TSubstream: AsyncRead + AsyncWrite + Send + Sync + 'static, // TODO: remove useless bounds
{
    type InEvent = Void;
    type OutEvent = IdentifySender<TSubstream>;
    type Substream = TSubstream;
    type Protocol = IdentifyProtocolConfig;
    type OutboundOpenInfo = ();

    #[inline]
    fn listen_protocol(&self) -> Self::Protocol {
        self.config.clone()
    }

    fn inject_fully_negotiated(
        &mut self,
        protocol: <Self::Protocol as ConnectionUpgrade<TSubstream>>::Output,
        endpoint: NodeHandlerEndpoint<Self::OutboundOpenInfo>,
    ) {
        match protocol {
            IdentifyOutput::Sender { sender } => {
                debug_assert!(if let NodeHandlerEndpoint::Listener = endpoint { true } else { false });
                self.pending_result.push(sender);
            }
            IdentifyOutput::RemoteInfo { .. } => unreachable!(
                "RemoteInfo can only be produced if we dial the protocol, but we never do that"
            ),
        }
    }

    #[inline]
    fn inject_event(&mut self, _: Self::InEvent) {}

    #[inline]
    fn inject_inbound_closed(&mut self) {}

    #[inline]
    fn inject_dial_upgrade_error(&mut self, _: Self::OutboundOpenInfo, _: io::Error) {}

    #[inline]
    fn shutdown(&mut self) {
        self.shutdown = true;
    }

    fn poll(
        &mut self,
    ) -> Poll<
        Option<
            ProtocolsHandlerEvent<
                Self::Protocol,
                Self::OutboundOpenInfo,
                Self::OutEvent,
            >,
        >,
        io::Error,
    > {
        if !self.pending_result.is_empty() {
            return Ok(Async::Ready(Some(ProtocolsHandlerEvent::Custom(
                self.pending_result.remove(0),
            ))));
        }

        if self.shutdown {
            Ok(Async::Ready(None))
        } else {
            Ok(Async::NotReady)
        }
    }
}
