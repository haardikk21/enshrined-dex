import { http, createConfig } from "wagmi";
import { defineChain } from "viem";

export const localChain = defineChain({
  id: 1337,
  name: "Local",
  nativeCurrency: {
    decimals: 18,
    name: "Ether",
    symbol: "ETH",
  },
  rpcUrls: {
    default: {
      http: ["http://localhost:8545"],
    },
  },
});

export const config = createConfig({
  chains: [localChain],
  transports: {
    [localChain.id]: http("http://localhost:8545"),
  },
});
