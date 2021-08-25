### TO-DO
- [ ] Ignore already used staking transactions
- [ ] Reward the block forgers
- [ ] Propagate transactions
- [ ] Remove transactions from the mempool that has been bundled on a propagated block
- [ ] Prioritize sending transactions to the next block forger
- [ ] Improve the election algorithm
- [x] Use `enum` instead of `struct` to easily have different types of Transactions inside the blocks 
- [ ] Make the discovery server a library and make a different crate to launch it as cli, this way the discovery server could be launched right from the simulation example