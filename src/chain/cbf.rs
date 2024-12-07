use std::sync::Arc;
use tokio::sync::Mutex;

use crate::Error;

use bitcoin::{block::Header, BlockHash};
use kyoto::{Client, IndexedBlock};
use tokio::sync::mpsc::{Sender, UnboundedReceiver};

type Height = u32;

#[derive(Debug)]
pub(crate) enum ChainRequest {
	Header(Height),
	Block(BlockHash),
}

#[derive(Debug)]
pub(crate) enum ChainEvent {
	Header(IndexedHeader),
	Block(IndexedBlock),
}

#[derive(Debug)]
pub(crate) struct IndexedHeader {
	pub(crate) height: Height,
	pub(crate) header: Header,
}

pub(crate) struct ChainFetcher {
	tx: Sender<ChainEvent>,
	rx: Arc<Mutex<UnboundedReceiver<ChainRequest>>>,
	client: Arc<Client>,
}

impl ChainFetcher {
	pub(crate) fn new(
		tx: Sender<ChainEvent>, rx: UnboundedReceiver<ChainRequest>, client: Arc<Client>,
	) -> Self {
		Self { tx, rx: Arc::new(Mutex::new(rx)), client }
	}

	pub(crate) async fn listen_for_requests(&self) -> Result<(), Error> {
		let mut rx = self.rx.lock().await;
		loop {
			if let Some(event) = rx.recv().await {
				match event {
					ChainRequest::Header(height) => {
						let header = self
							.client
							.get_header(height)
							.await
							.map_err(|_| Error::TxSyncFailed)?;
						self.tx
							.send(ChainEvent::Header(IndexedHeader { height, header }))
							.await
							.map_err(|_| Error::TxSyncFailed)?;
					},
					ChainRequest::Block(block_hash) => {
						let block = self
							.client
							.request_block(block_hash)
							.await
							.map_err(|_| Error::TxSyncFailed)?;
						self.tx
							.send(ChainEvent::Block(block))
							.await
							.map_err(|_| Error::TxSyncFailed)?;
					},
				}
			}
		}
	}
}
