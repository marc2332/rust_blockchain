A decentralized blockchain-based network platform.

Inspired by Bitcoin, Ethereum and Solana.

WIP.

| crate            | description                                                                                          |
|------------------|------------------------------------------------------------------------------------------------------|
| blockchain       | Different utilities to build a blockchain, blocks, hashing, public-private keys, signing...          |
| node             | Node implementation in Rust                                                                          |
| client           | JSON-RPC Client to connect and interact with a node                                                  |
| consensus        | Consensus utilities                                                                                  |
| discovery_server | A signaling server for nodes to find each other                                                      |
| cli              | CLI Runner of a Node                                                                                 |
| explorer         | A TUI app to display the evolution of the blockchain and monetary increase of the simulation nodes   |
| metrics_server   | A metrics server implementation in Rust                                                              |

## Requirements
- Cargo (nightly toolchain)

## Run simulation (WIP)
Run:
```shell
cd node
cargo run --example simulation --release
```

### TO-DO
- [x] Reward the block forgers
- [x] Propagate transactions
- [x] Remove transactions from the mempool that has been bundled on a propagated block
- [ ] Prioritize sending transactions to the next block forger
- [x] Improve the election algorithm
- [x] Use `enum` instead of `struct` to easily have different types of Transactions inside the blocks 
- [x] Make the discovery server a library,this way the discovery server could be launched right from the simulation example
- [ ] Synchronization support
- [ ] Fees ?
- [x] Transactions might be duplicated across different blocks if the network latency is too high, it should make sure the transactions  hasn't been already added to a block
- [x] Scalable block size relative to the network performance
- [x] Always have just the X last blocks of the blockchain on memory to avoid infinite memory increase
- [ ] Implement a consensus algorithm to prevent bad nodes to ignore certain transactions
- [ ] Improve simulation by allowing external nodes to join (this would need to have synchronization support)
- [x] Fix the never-ending memory usage increase
- [ ] Discard lost blocks after certain block height to avoid spam.
- [x] A metrics module that sends information such as new blocks or new txs, etc... through a WebSockets connection to an external point such as a server that could display this data in a frontend such as website.
- [ ] A website that shows information about a given metric server
- [ ] Option to get the current history of a wallet