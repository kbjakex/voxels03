## A quick overview of the layout of the project

### Views:
Views solve the problem of separating code/state that is totally unrelated to other code/state. The main menu needs very different state and logic than the in-game view: for example, once in game view, you need to have a connection to the server established.

This could be abstracted at a much higher level with dynamic dispatch, but this is a very simple, pragmatic and low-overhead way to accomplish the goal.

### Resources vs state:
Naming choices here are a bit arbitrary, but made in hopes of clarity.
* "resource" structs are pieces of state that are shared across the entire application (so these don't change across views, for example) and persist from the startup till the application is closed. Nothing here should be specific to any view or situation.

  Resources live in `resources.rs`.
* "state" structs represent any other state. The main menu view might have its own state, the in-game view has its own state. Any smaller components might have their own internal state, too.

  In other words, any state that _isn't_ relevant for the entire application. They may live wherever, whenever, but outside of `resources.rs`.

### The `utils` folder:
Any "enginey" code that isn't necessarily about this game specifically, but has to exist somewhere. Input management, frame pacing, compression, whatever.

Alternatively: things that need to exist, but are best kept
out of view for the most part to give room for the more important things, such as gameplay code.

Nothing in here has or should be written as 'generic, re-usable code' though, these are definitely still tailored for this game specifically and *not* expected to be re-used!

### The `renderer` crate:
Encapsulates anything related to communicating with the GPU. The hope is that outside of this, it is opaque which graphics API is even being used.

### The `netcode` crate:
Similarly, encapsulates all details about communicating with the server, such as set-up/teardown of the network thread, cross-thread communication, the underlying network protocol (QUIC) and sending/receiving of data. Both the netcode and the client crate can be aware of the game-specific protocol, but that should ideally be just about all the client has to care about.

### The `shared` crate (in parent directory):
The "shared" crate should be the home to everything that can be of use for both the client and the server.
