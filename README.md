```
  _______         ______              _______            __  
 /_  __(_)____   /_  __/___ ______   /_  __(_)________  / /_ 
  / / / / ___/    / / / __ `/ ___/    / / / / ___/ __ \/ __ \
 / / / / /__     / / / /_/ / /__     / / / / /  / /_/ / / / /
/_/ /_/\___/    /_/  \__,_/\___/    /_/ /_/_/   \____/_/ /_/ 
                                                             
```
# Tic-Tac-Tiroh
Tic-Tac-Tiroh is a **WIP** implementation of Tic Tac Toe over the [Iroh](https://iroh.computer) p2p protocol.

## Usage 
To play you first have to start the program without any arguments. This will cause the application to start waiting for connections to the display its NodeID.
```bash
cargo run
```

You can now give your peer that NodeID and specify it as the first argument.
```bash
cargo run <node-id>
```
