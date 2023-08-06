# WASM Multi Party ECDSA

This library provides a secure implementation of the Elliptic Curve Digital Signature Algorithm (ECDSA) in WebAssembly (WASM) implemented entirely in Rust. It enables parties to securely generate keys and sign messages using a threshold scheme without revealing their private keys. This solution outperforms previous hybrid approaches and is the first pure-wasm MPC solution.

[CoinFabrik](https://www.coinfabrik.com/) is the Web3 solutions company behind this implementation.

## Usage

### Installation

In order to use the library, you will need to install the following dependencies:

```shell
npm install @mpc-framework/wasm-multi-party-ecdsa comlink
```

### Preparation

As this library needs a web worker to perform the calculations, you'll need to create a worker instance. You can do this by creating a file called `worker.ts` and adding the following code:

```typescript
import init, { initThreadPool, MultiPartyEcdsa } from "wasm-multi-party-ecdsa";
import * as Comlink from "comlink";

void (async function () {
  // Needed for wasm-bindgen-rayon
  await init();
  await initThreadPool(1);

  // In case we want to add a hook to listen when it's ready
  self.postMessage({ ready: true });
})();

const worker = { MultiPartyEcdsa };
export type IWorker = typeof worker;

Comlink.expose(worker);
```

We'll be using [Comlink](https://github.com/GoogleChromeLabs/comlink) to communicate with the worker. This is a library that allows us to use web workers as if they were regular functions.

Finally, a temporary hack is needed for the `crypto.getRandomValues()` function to work with our library. More information can be found [here](). This code must be added to the same file `worker.ts`:

<!-- TODO: check if it really must be added there -->

```typescript
const getRandomValues = crypto.getRandomValues;
crypto.getRandomValues = function <T extends ArrayBufferView | null>(array: T) {
  const buffer = new Uint8Array(array as unknown as Uint8Array);
  const value = getRandomValues.call(crypto, buffer);
  (array as unknown as Uint8Array).set(value as unknown as Uint8Array);
  return array;
};
```

Now we're ready to start using the library.

### Keygen

Using the library is pretty straightforward. First we'll need to understand the concepts of groups, sessions and parties.

- **Groups** are a collection of parties that will hold a key. A group can be reused to generate multiple keys or sign multiple messages.

- **Sessions** are subgroups created by members of a group with the sole purpose of generating a key or signing a message. A session should not be reused.

- **Parties** are the members that will hold a key. Each party is identified by a unique number.

<!--  TODO: tell about the different notification messages available  -->

To generate our first set of keys, we'll define the number of parties and the threshold. The threshold is the minimum number of parties that need to be present in order to sign a message, minus one. The number of parties is the amount of parties that will hold a key.

> E.g.: Given a threshold of 1 and a number of parties of 3, we need at least 2 parties to be present in order to sign a message.

We can start by instantiating the library and connecting it to our [MPC-Manager](https://github.com/coinfabrik/mpc-manager) instance:

```typescript
import * as Comlink from "comlink";
import { IWorker } from "./worker";

// We need to create a new worker instance and wrap it with Comlink
const innerWorker = new Worker(new URL("./worker.ts", import.meta.url));
const worker = Comlink.wrap<IWorker>(innerWorker);

// Then we can instantiate the library. At this point we'll be connected
// to our manager.
const multiPartyEcdsa = await new worker.MultiPartyEcdsa("ws://localhost:8080");
```

Now we can create a new group and session, which we'll use to generate a new key:

```typescript
const NUMBER_OF_PARTIES = 3;
const THRESHOLD = 1;

const { group } = await multiPartyEcdsa.groupCreate(
  NUMBER_OF_PARTIES,
  THRESHOLD
);
const { session } = await multiPartyEcdsa.sessionCreate(
  group.id,
  "keygen",
  null
);
const { partyNumber } = await multiPartyEcdsa.sessionSignup(
  group.id,
  session.id
);
```

```typescript
// And use it to create a new key
const { localKey, publicKey } = await multiPartyEcdsa.keygen(
  group.id,
  session.id,
  partyNumber,
  NUMBER_OF_PARTIES,
  THRESHOLD
);
```

In the other clients:

```typescript
// ... rest of the code
const { group } = await multiPartyEcdsa.groupJoin(groupId);
const { session, partyNumber } = await multiPartyEcdsa.sessionSignup(
  groupId,
  sessionId
);
const { localKey, publicKey } = await multiPartyEcdsa.keygen(
  group.id,
  session.id,
  partyNumber,
  NUMBER_OF_PARTIES,
  THRESHOLD
);
```

And that's it! You now have a new multi-party key that can be used to sign messages.

### Signing

In order to sign a message, we'll need to create a new session:

```typescript
// ... rest of the code
const message = new Uint8Array([1, 2, 3]);
const parties = [1, 2];

const { session } = await multiPartyEcdsa.sessionCreate(
  groupId,
  "sign",
  message
);

// In this case we don't need to sign up, as we already got a party number
// assigned to us at the moment of the keygen.
await multiPartyEcdsa.sessionLogin(groupId, session.id, localKey.i);
const signature = await multiPartyEcdsa.sign(
  groupId,
  session.id,
  localKey,
  parties,
  message
);
```

In the other clients we must only login to the created session and wait for the signature to be generated:

```typescript
// ... rest of the code
const parties = [1, 2];

const { session } = await multiPartyEcdsa.sessionLogin(
  groupId,
  sessionId,
  localKey.i
);
const signature = await multiPartyEcdsa.sign(
  groupId,
  session.id,
  localKey,
  parties,
  message
);
```

And that's it! You have now signed your first message with a multi party threshold scheme.

## Contributing

If you'd like to contribute to the library, please open an issue or submit a pull request. We welcome any contributions, including bug fixes, feature requests, and documentation improvements.


## Acknowledgments

This project is based on the following projects:

- Rosario Gennaro and Steven Goldfeder - "One Round Threshold ECDSA with Identifiable Abort"(https://eprint.iacr.org/2020/540.pdf)
- ZenGo-X Multi Party Ecdsa (https://github.com/ZenGo-X/multi-party-ecdsa)
- LavaMoat TSS-Snap (https://github.com/LavaMoat/tss-snap)

## License

This project is licensed under the MIT License. Please see the LICENSE file for more information.
