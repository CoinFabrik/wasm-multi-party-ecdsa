import { webWorker } from "./web-worker";
import "./aio";
import * as Comlink from "comlink";
import { MultiPartyEcdsa } from "wasm-multi-party-ecdsa/wasm_multi_party_ecdsa";

/** @type {MultiPartyEcdsa} */
let mpc = null;

let groupId = null;
let sessionId = null;
let party = null;

const worker = Comlink.wrap(webWorker);

webWorker.addEventListener("message", (msg) => {
  if (msg.data.ready) {
    console.log("Worker ready!");
  }
});

document
  .getElementById("connect-button")
  .addEventListener("click", async () => {
    mpc = await new worker.Mpc("ws://localhost:8080");
    const a = (e) => {
      console.log("Session created", e);
    };

    mpc.onSessionCreated(Comlink.proxy(a));
  });

document
  .getElementById("create-group-button")
  .addEventListener("click", async () => {
    const res = await mpc.groupCreate(3, 1);
    console.log("groupCreate", res);
    groupId = res.group.id;
    updateGroup(res.group.id);
  });

document
  .getElementById("join-group-button")
  .addEventListener("click", async () => {
    const input = document.getElementById("join-group-input").value;
    if (!input) console.error("No group id provided");
    const res = await mpc.groupJoin(input);
    console.log("groupJoin", res);
    groupId = input;
    updateGroup(groupId);
  });

document
  .getElementById("create-session-button")
  .addEventListener("click", async () => {
    const res = await mpc.sessionCreate(groupId, "keygen", null);
    console.log("sessionCreate", res);
    updateSession(res.session.id);
    document.getElementById("join-session-input").value = sessionId;
  });

document
  .getElementById("join-session-button")
  .addEventListener("click", async () => {
    const input = document.getElementById("join-session-input").value;
    if (!input) console.error("No session id provided");
    const res = await mpc.sessionSignup(groupId, input);
    console.log("sessionJoin", res);
    party = res.partyNumber;
    sessionId = input;
    updateSession(sessionId);
  });

document.getElementById("keygen").addEventListener("click", async () => {
  let response = await mpc.keygen(groupId, sessionId, party, 3, 1);
  console.log("Keygen", response);
  document.getElementById("keygen-output").innerHTML = response;
});

const updateGroup = (data) => {
  document.getElementById(
    "group-label"
  ).textContent = `Connected to group: ${data}`;
};

const updateSession = (data) => {
  document.getElementById(
    "session-label"
  ).textContent = `Connected to session: ${data}`;
};
