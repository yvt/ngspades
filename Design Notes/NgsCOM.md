# NgsCOM

## Uncurated ideas

- Do away with a COM-compatible ABI and introduce fat interface pointers, which provide various benefits:
    - The implementation macro on the Rust side will be much simpler
    - Opens a possibility to inline small objects into interface pointers on some situations (e.g., `Arc` and CCW)


