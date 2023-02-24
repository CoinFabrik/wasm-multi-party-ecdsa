import { webWorker } from "./web-worker";
import * as Comlink from "comlink";
import { MultiPartyEcdsa } from "wasm-multi-party-ecdsa/wasm_multi_party_ecdsa";

const N = 3;
const T = 1;
/** @type {MultiPartyEcdsa[]} */
let mpc = new Array(N);
let party = new Array(N);
let keys = new Array(N);
let groupId = null;

const worker = Comlink.wrap(webWorker);

document
  .getElementById("aio-connect-button")
  .addEventListener("click", async () => {
    console.log(`Initializing ${N} clients`);
    for (let i = 0; i < N; i++) {
      mpc[i] = await new worker.Mpc("ws://localhost:8080");
    }
  });

document
  .getElementById("aio-start-keygen-button")
  .addEventListener("click", async () => {
    console.log("Starting keygen process");
    const start = performance.now();

    // Creating group in client 1 and joining others
    const createGroupRes = await mpc[0].groupCreate(N, T);
    groupId = createGroupRes.group.id;
    console.log("Groupid", groupId);
    for (let i = 1; i < N; i++) {
      await mpc[i].groupJoin(groupId);
    }

    // Creating session with client 1 and joining all
    const createSessionRes = await mpc[0].sessionCreate(
      groupId,
      "keygen",
      null
    );
    const sessionId = createSessionRes.session.id;
    console.log("Session", sessionId);
    for (let i = 0; i < N; i++) {
      const sessionSignupRes = await mpc[i].sessionSignup(groupId, sessionId);
      party[i] = sessionSignupRes.partyNumber;
    }

    // Starting keygen process
    keys = await Promise.all(
      mpc.map((client, i) => client.keygen(groupId, sessionId, party[i], N, T))
    );

    const end = performance.now();
    console.log("Time taken: " + (end - start) + " milliseconds.");

    console.log("Keys", keys);
  });

document
  .getElementById("aio-start-sign-button")
  .addEventListener("click", async () => {
    if (groupId === null) {
      alert("Please run keygen first");
      return;
    }

    const input = document.getElementById("aio-start-sign-input").value;
    if (!input) {
      alert("Please provide a message to sign");
      return;
    }

    // parse hex string to uint8array
    const inputBytes = new Uint8Array(
      input.match(/.{1,2}/g).map((byte) => parseInt(byte, 16))
    );

    console.log("Starting signature process");
    const start = performance.now();

    // Creating session with client 1 and joining only client 2
    const createSessionRes = await mpc[0].sessionCreate(groupId, "sign", input);
    const sessionId = createSessionRes.session.id;
    console.log("Session", sessionId);
    for (let i = 0; i < T + 1; i++) {
      await mpc[i].sessionLogin(groupId, sessionId, keys[i].localKey.i);
    }

    // Starting signature process
    const parties = keys.slice(0, T + 1).map((key) => key.localKey.i);
    const signatures = await Promise.all(
      mpc
        .slice(0, T + 1)
        .map((client, i) =>
          client.sign(groupId, sessionId, keys[i].localKey, parties, inputBytes)
        )
    );

    const end = performance.now();
    console.log("Time taken: " + (end - start) + " milliseconds.");

    console.log("Signatures", signatures);
  });
