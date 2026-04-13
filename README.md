# dijavu

A minimal, yet extensible, type-safe dependency injection system for Rust, which enables modular architectures with 
easily composable injectables.
There is a clear split between the _initialization_ and _runtime_ phase of the application, which work on often
similarly structured, but different objects (hence the name).

In the initialization phase, you operate on a mutable data container to incrementally set up your application
components.
After the initialization phase, the final and immutable app state is constructed, which can then be passed around the
application using a cheap copyable handle.
This handle is created by leaking a data container to the `'static` lifetime, which makes this library only suitable for
applications with a single initialization process during runtime.

## Features

- `auto_init_default`: Automatically initialize `InitInjectable` types when trait is implemented via the derive macro (e.g. `#[inject(init(auto))]` always set)
