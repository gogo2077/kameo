# Kameo Remote Actor Protocol

This document describes the network protocol used by Kameo for remote actor communication. This specification is intended for developers implementing Kameo-compatible clients or servers in other languages (e.g., Go, JavaScript, Python).

## Overview

Kameo uses [libp2p](https://libp2p.io/) for peer-to-peer networking. The system consists of two main components:

1.  **Registry**: A distributed actor registry built on Kademlia DHT.
2.  **Messaging**: A direct request-response protocol for actor-to-actor communication.

## Protocol IDs

Kameo uses the following default libp2p protocol IDs:

| Component | Protocol ID | Description |
| :--- | :--- | :--- |
| **Registry** | `/kameo/registry/kad/1.0.0` | Kademlia DHT for actor discovery and metadata storage. |
| **Messaging** | `/kameo/messaging/1.0.0` | Direct request-response protocol for sending messages to actors. |

> **Note**: These protocol IDs can be configured in Kameo. Ensure your implementation matches the configuration of the Kameo node you are connecting to.

## 1. Actor Registration & Discovery (DHT)

The registry allows actors to register themselves under a string name (e.g., `"my_service"`) and be discovered by other peers. This is achieved using the Kademlia DHT.

### Registration Flow

To register an actor named `actor_name` hosted on `peer_id`, a node must perform two operations in the DHT:

1.  **Provide the Actor Name**: Advertise that the local peer provides the key `actor_name`.
    *   **Libp2p Operation**: `PROVIDER` (StartProviding)
    *   **Key**: `actor_name` (raw bytes of the UTF-8 string)

2.  **Store Metadata**: Store the actor's metadata record in the DHT.
    *   **Libp2p Operation**: `PUT_VALUE` (PutRecord)
    *   **Key**: `{actor_name}:meta:{peer_id}` (UTF-8 string)
    *   **Value**: Serialized `ActorRegistration` data (see below).

### Metadata Format (`ActorRegistration`)

The value stored in the DHT under the `{actor_name}:meta:{peer_id}` key is a binary blob with the following structure:

| Field | Size (bytes) | Type | Description |
| :--- | :--- | :--- | :--- |
| `peer_id_length` | 1 | `u8` | Length of the Peer ID bytes ($N$). |
| `actor_sequence_id` | 8 | `u64` (Little Endian) | The local unique ID of the actor on its host node. |
| `peer_id` | $N$ | `[u8; N]` | The raw bytes of the `PeerId`. |
| `actor_remote_id` | $M$ | `[u8; M]` | The UTF-8 bytes of the actor's remote type identifier string. |

#### Example: `ActorRegistration` Byte Layout

If:
*   `peer_id` is `12D3KooW...` (34 bytes raw)
*   `actor_sequence_id` is `42` (`0x2A`)
*   `actor_remote_id` is `"my_actor"`

The payload would be:

```text
[0x22]                                      // peer_id_length (34)
[0x2A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00] // actor_sequence_id (42, LE)
[0x00, 0x24, 0x08, 0x01, ...]               // peer_id bytes (34 bytes)
[0x6D, 0x79, 0x5F, 0x61, 0x63, 0x74, 0x6F, 0x72] // "my_actor" (UTF-8)
```

### Discovery Flow (Lookup)

To find an actor named `actor_name`:

1.  **Get Providers**: Query the DHT for peers providing `actor_name`.
    *   **Libp2p Operation**: `GET_PROVIDERS`
    *   **Key**: `actor_name`
    *   **Result**: A list of `PeerId`s.

2.  **Get Metadata**: For each discovered `PeerId`, query the DHT for the metadata record.
    *   **Libp2p Operation**: `GET_VALUE`
    *   **Key**: `{actor_name}:meta:{peer_id}`
    *   **Result**: The binary `ActorRegistration` blob described above.

---

## 2. Remote Messaging

Once an actor's location (`PeerId` and `ActorId`) is known, messages are sent directly using the messaging protocol.

*   **Transport**: Libp2p Request-Response
*   **Protocol ID**: `/kameo/messaging/1.0.0`
*   **Serialization**: [MessagePack](https://msgpack.org/) (via `rmp-serde`)

### Request Structure (`SwarmRequest`)

Requests are serialized as a MessagePack Enum. The most common request types are `Ask` (expects response) and `Tell` (fire-and-forget).

#### Enum Variants

Note: The integer values below represent the MessagePack enum variant index if serialized as an integer (default in Rust serde for enums is usually map/string, but Kameo uses `rmp-serde` which may default to integer or string variants depending on configuration. **Kameo uses struct/map-based serialization for the enum variants**).

The `SwarmRequest` is serialized as a MessagePack map (or array depending on serde settings, usually map for self-describing formats).

**`Ask` Request:**

```json
{
  "Ask": {
    "actor_id": <ActorId>,
    "actor_remote_id": "remote_id_string",
    "message_remote_id": "message_id_string",
    "payload": <Binary>,
    "mailbox_timeout": <Option<Duration>>,
    "reply_timeout": <Option<Duration>>,
    "immediate": <Boolean>
  }
}
```

**`Tell` Request:**

```json
{
  "Tell": {
    "actor_id": <ActorId>,
    "actor_remote_id": "remote_id_string",
    "message_remote_id": "message_id_string",
    "payload": <Binary>,
    "mailbox_timeout": <Option<Duration>>,
    "immediate": <Boolean>
  }
}
```

#### Fields Description

*   **`actor_id`**: The `ActorId` struct.
    *   Serialized as: `[sequence_id (u64), peer_id (Option<PeerId>)]` or specific struct serialization. *Check `src/actor/id.rs` for exact `serde` impl.*
*   **`actor_remote_id`**: The unique string ID of the actor type (e.g., `"my_crate::MyActor"`).
*   **`message_remote_id`**: The unique string ID of the message type (e.g., `"my_crate::MyMessage"`).
*   **`payload`**: The message content, serialized via MessagePack. **Note**: This is a nested binary blob. The message itself is MessagePack-encoded, and that byte array is passed here.
*   **`mailbox_timeout` / `reply_timeout`**: `Option<Duration>`.
    *   `Duration` is typically serialized as `[secs (u64), nanos (u32)]`.

### Response Structure (`SwarmResponse`)

Responses are also MessagePack Enums.

**`Ask` Response:**

```json
{
  "Ask": {
    "Ok": <Binary> // Successful reply payload (MessagePack encoded)
  }
}
```
*OR*
```json
{
  "Ask": {
    "Err": <RemoteSendError>
  }
}
```

**`Tell` Response:**

```json
{
  "Tell": {
    "Ok": null
  }
}
```

## Cross-Language Implementation Tips (e.g., Go)

1.  **DHT Configuration**: Ensure your Kademlia DHT implementation uses the protocol ID `/kameo/registry/kad/1.0.0`. Standard Go `go-libp2p-kad-dht` often defaults to `/ipfs/kad/1.0.0` but supports protocol customization.
2.  **Metadata Key**: When using `PutValue`, ensure the key string `{actor_name}:meta:{peer_id}` is exactly preserved.
3.  **Endianness**: The `actor_sequence_id` in the metadata blob must be **Little Endian**.
4.  **MessagePack**: Use a compliant MessagePack library. Be careful with how enums are serialized (as maps with a single key vs integers). Kameo uses Rust's `serde` default for enums (usually `{ "VariantName": { ...fields... } }`).

## Future Work

*   Standardization of the handshake and capabilities exchange.
*   Formal schema definition (e.g., Protobuf) for messaging to replace direct MessagePack constructs.
