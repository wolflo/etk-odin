Example of low-level evm bytecode testing using [evmodin](https://github.com/vorot93/evmodin.git) to set the environment and execute the bytecode.

Code is assembled with [etk-asm](https://github.com/quilt/etk), then stack, memory, storage, etc. can all be set before executing.

[Huff](https://github.com/AztecProtocol/huff) used to have some tooling for this, but it involved prepending bytecode that used `PUSH` and `MSTORE` instructions to set the initial environment.
This had some limitations, including inability to set execution address or returndata, and changing the result of instructions like `GAS` and `CODESIZE`.
