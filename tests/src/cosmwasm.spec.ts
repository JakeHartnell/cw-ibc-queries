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
  const { contractAddress: wasmCont } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    wasmIds.controller,
    {},
    "simple controller",
    "auto"
  );
  t.truthy(wasmCont);
  const { ibcPortId: controllerPort } = await wasmClient.sign.getContract(wasmCont);
  t.log(`Controller Port: ${controllerPort}`);
  assert(controllerPort);

  // instantiate ica host on osmosis
  const osmoClient = await setupOsmosisClient();
  const { contractAddress: osmoHost } = await osmoClient.sign.instantiate(
    osmoClient.senderAddress,
    osmosisIds.host,
    {},
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
  channelIds: {
    wasm: string;
    osmo: string;
  };
}

async function demoSetup(): Promise<SetupInfo> {
  // instantiate ica controller on wasmd
  const wasmClient = await setupWasmClient();
  const { contractAddress: wasmController } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    wasmIds.controller,
    {},
    "IBC Queries contract",
    "auto"
  );
  const { ibcPortId: controllerPort } = await wasmClient.sign.getContract(wasmController);
  assert(controllerPort);

  // instantiate ica host on osmosis
  const osmoClient = await setupOsmosisClient();
  const { contractAddress: osmoHost } = await osmoClient.sign.instantiate(
    osmoClient.senderAddress,
    osmosisIds.host,
    {},
    "IBC Queries contract",
    "auto"
  );
  const { ibcPortId: hostPort } = await osmoClient.sign.getContract(osmoHost);
  assert(hostPort);

  // create a connection and channel for simple-ica
  const [src, dest] = await setup(wasmd, osmosis);
  const link = await Link.createWithNewConnections(src, dest);
  const channelInfo = await link.createChannel("A", controllerPort, hostPort, Order.ORDER_UNORDERED, IbcVersion);
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
    wasmController,
    osmoHost,
    link,
    ics20,
    channelIds,
  };
}

test.serial("query remote chain", async (t) => {
  const { osmoClient, wasmClient, wasmController, link, channelIds } = await demoSetup();

  // Use IBC queries to query info from the remote contract
  const ibcQuery = await wasmClient.sign.execute(
    wasmClient.senderAddress,
    wasmController,
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
      },
    },
    "auto"
  );
  console.log(ibcQuery);

  // relay this over
  const info = await link.relayAll();
  console.log(info);
  console.log(fromUtf8(info.acksFromB[0].acknowledgement));
  // assertPacketsFromA(info1, 1, true);

  const result = await wasmClient.sign.queryContractSmart(wasmController, {
    latest_query_result: {
      channel_id: channelIds.wasm,
    },
  });

  console.log(result);
  console.log(fromUtf8(fromBase64(result.response.acknowledgement.data)));
  t.truthy(result);

  // // Use IBC queries to query info from the remote contract
  // const ibcQuery = await wasmClient.sign.execute(
  //   wasmClient.senderAddress,
  //   wasmController,
  //   {
  //     ibc_query: {
  //       channel_id: channelIds.wasm,
  //       msgs: [
  //         {
  //           wasm: {
  //             smart: {
  //               msg: toBase64(toUtf8(JSON.stringify({ latest_query_result: { channel_id: channelIds.osmo } }))),
  //               contract_addr: osmoHost,
  //             },
  //           },
  //         },
  //       ],
  //     },
  //   },
  //   "auto"
  // );
  // // relay this over
  // const info = await link.relayAll();
  // assertPacketsFromA(info, 1, true);
});
