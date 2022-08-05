import { CosmWasmSigner, Link, testutils } from "@confio/relayer";
import { toBase64, toUtf8 } from "@cosmjs/encoding";
import { assert } from "@cosmjs/utils";
import test from "ava";
import { Order } from "cosmjs-types/ibc/core/channel/v1/channel";

const { osmosis: oldOsmo, setup, wasmd, randomAddress } = testutils;
const osmosis = { ...oldOsmo, minFee: "0.025uosmo" };

import {
  checkRemoteBalance,
  fundRemoteAccount,
  listAccounts,
  remoteBankMultiSend,
  remoteBankSend,
  showAccount,
} from "./controller";
import {
  assertPacketsFromA,
  IbcVersion,
  parseAcknowledgementSuccess,
  setupContracts,
  setupOsmosisClient,
  setupWasmClient,
} from "./utils";

let wasmIds: Record<string, number> = {};
let osmosisIds: Record<string, number> = {};

test.before(async (t) => {
  console.debug("Upload contracts to wasmd...");
  const wasmContracts = {
    controller: "./internal/cw_ibc_queries.wasm",
  };
  const wasmSign = await setupWasmClient();
  wasmIds = await setupContracts(wasmSign, wasmContracts);

  console.debug("Upload contracts to osmosis...");
  const osmosisContracts = {
    host: "./internal/cw_ibc_queries.wasm",
  };
  const osmosisSign = await setupOsmosisClient();
  osmosisIds = await setupContracts(osmosisSign, osmosisContracts);

  t.pass();
});

test.serial("set up channel with ibc-queries contract", async (t) => {
  // instantiate cw-ibc-queries on wasmd
  const wasmClient = await setupWasmClient();
  const initController = {};
  const { contractAddress: wasmCont } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    wasmIds.controller,
    initController,
    "simple controller",
    "auto"
  );
  t.truthy(wasmCont);
  const { ibcPortId: controllerPort } = await wasmClient.sign.getContract(wasmCont);
  t.log(`Controller Port: ${controllerPort}`);
  assert(controllerPort);

  // instantiate ica host on osmosis
  const osmoClient = await setupOsmosisClient();
  const initHost = {
    cw1_code_id: osmosisIds.whitelist,
  };
  const { contractAddress: osmoHost } = await osmoClient.sign.instantiate(
    osmoClient.senderAddress,
    osmosisIds.host,
    initHost,
    "simple host",
    "auto"
  );
  t.truthy(osmoHost);
  const { ibcPortId: hostPort } = await osmoClient.sign.getContract(osmoHost);
  t.log(`Host Port: ${hostPort}`);
  assert(hostPort);

  const [src, dest] = await setup(wasmd, osmosis);
  const link = await Link.createWithNewConnections(src, dest);
  await link.createChannel("A", controllerPort, hostPort, Order.ORDER_UNORDERED, IbcVersion);
});

interface SetupInfo {
  wasmClient: CosmWasmSigner;
  osmoClient: CosmWasmSigner;
  wasmController: string;
  osmoHost: string;
  link: Link;
  ics20: {
    wasm: string;
    osmo: string;
  };
}

async function demoSetup(): Promise<SetupInfo> {
  // instantiate ica controller on wasmd
  const wasmClient = await setupWasmClient();
  const initController = {};
  const { contractAddress: wasmController } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    wasmIds.controller,
    initController,
    "IBC Queries contract",
    "auto"
  );
  const { ibcPortId: controllerPort } = await wasmClient.sign.getContract(wasmController);
  assert(controllerPort);

  // instantiate ica host on osmosis
  const osmoClient = await setupOsmosisClient();
  const initHost = {
    cw1_code_id: osmosisIds.whitelist,
  };
  const { contractAddress: osmoHost } = await osmoClient.sign.instantiate(
    osmoClient.senderAddress,
    osmosisIds.host,
    initHost,
    "IBC Queries contract",
    "auto"
  );
  const { ibcPortId: hostPort } = await osmoClient.sign.getContract(osmoHost);
  assert(hostPort);

  // create a connection and channel for simple-ica
  const [src, dest] = await setup(wasmd, osmosis);
  const link = await Link.createWithNewConnections(src, dest);
  await link.createChannel("A", controllerPort, hostPort, Order.ORDER_UNORDERED, IbcVersion);

  // also create a ics20 channel on this connection
  const ics20Info = await link.createChannel("A", wasmd.ics20Port, osmosis.ics20Port, Order.ORDER_UNORDERED, "ics20-1");
  const ics20 = {
    wasm: ics20Info.src.channelId,
    osmo: ics20Info.dest.channelId,
  };

  return {
    wasmClient,
    osmoClient,
    wasmController,
    osmoHost,
    link,
    ics20,
  };
}

test.serial("query remote chain", async (t) => {
  const { wasmClient, wasmController, link, osmoClient, osmoHost } = await demoSetup();

  // there is an initial packet to relay for the whoami run
  let info = await link.relayAll();
  assertPacketsFromA(info, 1, true);

  // get the account info
  const accounts = await listAccounts(wasmClient, wasmController);
  t.is(accounts.length, 1);
  const { remote_addr: remoteAddr, channel_id: channelId } = accounts[0];
  assert(remoteAddr);
  assert(channelId);

  // send some osmo to the remote address (using another funded account there)
  const initFunds = { amount: "2500600", denom: osmosis.denomFee };
  await osmoClient.sign.sendTokens(osmoClient.senderAddress, remoteAddr, [initFunds], "auto");

  // make a new empty account on osmosis
  const emptyAddr = randomAddress(osmosis.prefix);
  const noFunds = await osmoClient.sign.getBalance(emptyAddr, osmosis.denomFee);
  t.is(noFunds.amount, "0");

  // from wasmd, send a packet to transfer funds from remoteAddr to emptyAddr
  const sendFunds = { amount: "1200300", denom: osmosis.denomFee };
  await remoteBankSend(wasmClient, wasmController, channelId, emptyAddr, [sendFunds]);

  // relay this over
  info = await link.relayAll();
  assertPacketsFromA(info, 1, true);
  // TODO: add helper for this
  const contractData = parseAcknowledgementSuccess(info.acksFromB[0]);
  // check we get { results : ['']} (one message with no data)
  t.deepEqual(contractData, { results: [""] });

  // ensure that the money was transfered
  const gotFunds = await osmoClient.sign.getBalance(emptyAddr, osmosis.denomFee);
  t.deepEqual(gotFunds, sendFunds);

  // Use IBC queries to query account info from the remote contract
  const ibcQuery = await wasmClient.sign.execute(
    wasmClient.senderAddress,
    wasmController,
    {
      ibc_query: {
        channel_id: channelId,
        msgs: [{ smart: { msg: toBase64(toUtf8(JSON.stringify({ list_accounts: {} }))), contract_addr: osmoHost } }],
      },
    },
    "auto"
  );
  console.log(ibcQuery);
});
