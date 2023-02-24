import init, {
  initThreadPool,
  MultiPartyEcdsa as Mpc,
} from "wasm-multi-party-ecdsa/wasm_multi_party_ecdsa";
import * as Comlink from "comlink";

// Temporary hack for getRandomValues() error
const getRandomValues = crypto.getRandomValues;
crypto.getRandomValues = function (array) {
  const buffer = new Uint8Array(array);
  const value = getRandomValues.call(crypto, buffer);
  array.set(value);
  return array;
};

console.log("Worker is initializing...");
void (async function () {
  await init();
  await initThreadPool(1);
  self.postMessage({ ready: true });
})();

Comlink.expose({
  Mpc,
});
