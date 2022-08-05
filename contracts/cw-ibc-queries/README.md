# CosmWasm IBC Queries

Allows for querying the state of other CosmWasm chains using IBC. Support can be added for other IBC Query implementations once specifications for those implementations have been finalized.

## Workflow

Requires a `cw-ibc-queries` contract on both chains.

## Protocol

The packets sent look like:

```rust
pub enum PacketMsg {
  IbcQuery { msgs: Vec<QueryRequest>, callback: Option<String> },
}
```

Success looks like:

``` json
TODO
```

The error ack packet always looks like this:

```json
{
  "error": "invalid packet: <detailed error message>"
}
```
