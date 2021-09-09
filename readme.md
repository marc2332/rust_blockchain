This projects intends to make possible to have distributed blockchain in which blocks authors are automatically picked based on facts all the nodes know, this makes it impossible to propagate "wrong" blocks among nodes that follow the protocol.

WIP.

| crate            | description                                                                                 |
|------------------|---------------------------------------------------------------------------------------------|
| blockchain       | Different utilities to build a blockchain, blocks, hashing, public-private keys, signing... |
| node             | Node client that runs a JSON-RPC server and has a blockchain (WIP)                          |
| client           | JSON-RPC Client to connect and interact with a node                                         |
| consensus        | Consensus utilities                                                                         |
| discovery_server | A signaling server for nodes to find each other                                             |
| cli              | CLI Runner of a Node                                                                        |

## Requirements
- Cargo (nightly toolchain)
- Openssl

## Run simulation (WIP)
Run:
```shell
cd node
cargo run --example simulation
```