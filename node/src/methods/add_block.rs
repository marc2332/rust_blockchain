use std::sync::{
    Arc,
    Mutex,
};

use crate::{
    mempool::Mempool,
    NodeState,
};
use blockchain::{
    Block,
    PublicAddress,
};
use client::RPCClient;

pub async fn add_block(state: &Arc<Mutex<NodeState>>, block: Block) {
    let is_block_ok = {
        /*
         * Make sure the the signer is the block creator by verifying the block
         * signature using the creator's public key.
         * Also, make sure of the veracity of the incoming transactions by checking
         * that it's hash, history, and ammount are correct according the current chainstate.
         * If not, the block will be saved into the lost blocks list, and everytime there is a new incoming block,
         * this lost block will be tried to be added. Having a lost block might be due to latency.
         */
        if block.verify_sign_with(&PublicAddress::from(&block.key)) {
            let mut chainstate = state.lock().unwrap().blockchain.state.clone();
            Mempool::verify_veracity_of_incoming_transactions(&block.transactions, &mut chainstate)
        } else {
            false
        }
    };

    if is_block_ok {
        if state
            .lock()
            .unwrap()
            .blockchain
            .add_block(&block.clone())
            .is_ok()
        {
            let mut state = state.lock().unwrap();
            // Elect the next forger
            state.elect_new_forger();

            // Remove the block transactions from the mempool
            for tx in &block.transactions {
                state.mempool.remove_transaction(&tx.get_hash())
            }
        }
    } else {
        let state = state.clone();

        let peers = state.lock().unwrap().peers.clone();
        let prev_hash = state
            .lock()
            .unwrap()
            .blockchain
            .last_block_hash
            .as_ref()
            .unwrap()
            .unite();

        // Ask known peers for the missing block
        for (hostname, rpc_port, _) in peers.values() {
            let hostname = hostname.clone();
            let rpc_port = *rpc_port;

            let client = RPCClient::new(&format!("http://{}:{}", hostname, rpc_port))
                .await
                .unwrap();

            let remote_block = client.get_block_with_prev_hash(prev_hash.clone()).await;

            if let Ok(Some(block)) = remote_block {
                state
                    .lock()
                    .unwrap()
                    .lost_blocks
                    .insert(block.hash.unite(), block);
                break;
            }
        }

        // Added the new block to the lost queue
        state
            .lock()
            .unwrap()
            .lost_blocks
            .insert(block.hash.unite(), block);

        tracing::warn!(
            "(Node.{}) Tried to add a broken block.",
            state.lock().unwrap().id
        );
    }

    let mut state = state.lock().unwrap();

    /*
     * Blockchain regeneration
     * This tries to append previously lost blocks (probably due to latency) into the chain
     */
    if !state.lost_blocks.is_empty() {
        let mut blocks_iter = state.lost_blocks.clone().into_iter().peekable();
        let mut blocks = state.lost_blocks.clone();
        let mut any_recovered_block = false;

        while blocks_iter.peek().is_some() {
            let (_, block) = blocks_iter.next().unwrap();
            let is_block_ok = state.blockchain.add_block(&block.clone()).is_ok();
            if is_block_ok {
                blocks.remove(&block.hash.unite());
                blocks_iter = blocks.clone().into_iter().peekable();

                any_recovered_block = true;

                // Remove confirmed transactions from the mempool
                for tx in &block.transactions {
                    state.mempool.remove_transaction(&tx.get_hash());
                }
            }
        }

        // If any block has been recovered then elect a new forger
        if any_recovered_block {
            state.elect_new_forger();
            state.lost_blocks = blocks;
        }

        tracing::warn!(
            "(Node.{}) Length of lost blocks is <{}>",
            state.id,
            state.lost_blocks.len()
        );
    }
}
