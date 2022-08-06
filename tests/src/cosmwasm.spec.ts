import { CosmWasmSigner, Link, testutils } from "@confio/relayer";
// import { toBase64, toUtf8 } from "@cosmjs/encoding";
import { fromBase64, fromUtf8 } from "@cosmjs/encoding";
import { assert } from "@cosmjs/utils";
import test from "ava";
import { Order } from "cosmjs-types/ibc/core/channel/v1/channel";

const { osmosis: oldOsmo, setup, wasmd } = testutils;
const osmosis = { ...oldOsmo, minFee: "0.025uosmo" };

import { IbcVersion, setupContracts, setupOsmosisClient, setupWasmClient } from "./utils";

let wasmIds: Record<string, number> = {};
let osmosisIds: Record<string, number> = {};

test.before(async (t) => {
  console.debug("Upload contracts to wasmd...");
  const wasmContracts = {
    querier: "./internal/cw_ibc_queries.wasm",
    receiver: "./internal/cw_ibc_query_receiver.wasm",
  };
  const wasmSign = await setupWasmClient();
  wasmIds = await setupContracts(wasmSign, wasmContracts);

  console.debug("Upload contracts to osmosis...");
  const osmosisContracts = {
    querier: "./internal/cw_ibc_queries.wasm",
  };
  const osmosisSign = await setupOsmosisClient();
  osmosisIds = await setupContracts(osmosisSign, osmosisContracts);

  t.pass();
});

test.serial("set up channel with ibc-queries contract", async (t) => {
  // instantiate cw-ibc-queries on wasmd
  const wasmClient = await setupWasmClient();
  const { contractAddress: wasmCont } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    wasmIds.querier,
    {},
    "simple querier",
    "auto"
  );
  t.truthy(wasmCont);
  const { ibcPortId: wasmQuerierPort } = await wasmClient.sign.getContract(wasmCont);
  t.log(`Querier Port: ${wasmQuerierPort}`);
  assert(wasmQuerierPort);

  // instantiate ica querier on osmosis
  const osmoClient = await setupOsmosisClient();
  const { contractAddress: osmoQuerier } = await osmoClient.sign.instantiate(
    osmoClient.senderAddress,
    osmosisIds.querier,
    {},
    "simple querier",
    "auto"
  );
  t.truthy(osmoQuerier);
  const { ibcPortId: osmoQuerierPort } = await osmoClient.sign.getContract(osmoQuerier);
  t.log(`Querier Port: ${osmoQuerierPort}`);
  assert(osmoQuerierPort);

  const [src, dest] = await setup(wasmd, osmosis);
  const link = await Link.createWithNewConnections(src, dest);
  await link.createChannel("A", wasmQuerierPort, osmoQuerierPort, Order.ORDER_UNORDERED, IbcVersion);
});

interface SetupInfo {
  wasmClient: CosmWasmSigner;
  osmoClient: CosmWasmSigner;
  wasmQuerier: string;
  osmoQuerier: string;
  wasmQueryReceiver: string;
  link: Link;
  ics20: {
    wasm: string;
    osmo: string;
  };
  channelIds: {
    wasm: string;
    osmo: string;
  };
}

async function demoSetup(): Promise<SetupInfo> {
  // instantiate ica querier on wasmd
  const wasmClient = await setupWasmClient();
  const { contractAddress: wasmQuerier } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    wasmIds.querier,
    {},
    "IBC Queries contract",
    "auto"
  );
  const { ibcPortId: wasmQuerierPort } = await wasmClient.sign.getContract(wasmQuerier);
  assert(wasmQuerierPort);

  // instantiate ibc query receiver on wasmd
  const { contractAddress: wasmQueryReceiver } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    wasmIds.receiver,
    {},
    "IBC Query receiver contract",
    "auto"
  );
  assert(wasmQueryReceiver);

  // instantiate ica querier on osmosis
  const osmoClient = await setupOsmosisClient();
  const { contractAddress: osmoQuerier } = await osmoClient.sign.instantiate(
    osmoClient.senderAddress,
    osmosisIds.querier,
    {},
    "IBC Queries contract",
    "auto"
  );
  const { ibcPortId: osmoQuerierPort } = await osmoClient.sign.getContract(osmoQuerier);
  assert(osmoQuerierPort);

  // create a connection and channel for simple-ica
  const [src, dest] = await setup(wasmd, osmosis);
  const link = await Link.createWithNewConnections(src, dest);
  const channelInfo = await link.createChannel(
    "A",
    wasmQuerierPort,
    osmoQuerierPort,
    Order.ORDER_UNORDERED,
    IbcVersion
  );
  const channelIds = {
    wasm: channelInfo.src.channelId,
    osmo: channelInfo.src.channelId,
  };

  // also create a ics20 channel on this connection
  const ics20Info = await link.createChannel("A", wasmd.ics20Port, osmosis.ics20Port, Order.ORDER_UNORDERED, "ics20-1");
  const ics20 = {
    wasm: ics20Info.src.channelId,
    osmo: ics20Info.dest.channelId,
  };
  console.log(ics20Info);

  return {
    wasmClient,
    osmoClient,
    wasmQuerier,
    osmoQuerier,
    wasmQueryReceiver,
    link,
    ics20,
    channelIds,
  };
}

test.serial("query remote chain", async (t) => {
  const { osmoClient, wasmClient, wasmQuerier, link, channelIds, wasmQueryReceiver } = await demoSetup();

  // Use IBC queries to query info from the remote contract
  const ibcQuery = await wasmClient.sign.execute(
    wasmClient.senderAddress,
    wasmQuerier,
    {
      ibc_query: {
        channel_id: channelIds.wasm,
        msgs: [
          {
            bank: {
              all_balances: {
                address: osmoClient.senderAddress,
              },
            },
          },
        ],
        callback: wasmQueryReceiver,
      },
    },
    "auto"
  );
  console.log(ibcQuery);

  // relay this over
  const info = await link.relayAll();
  console.log(info);
  console.log(fromUtf8(info.acksFromB[0].acknowledgement));

  const result = await wasmClient.sign.queryContractSmart(wasmQueryReceiver, {
    latest_query_result: {
      channel_id: channelIds.wasm,
    },
  });

  console.log(result);
  console.log(fromUtf8(fromBase64(result.response.acknowledgement.data)));
  t.truthy(result);
});
