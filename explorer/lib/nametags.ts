/**
 * Address nametags for the explorer
 */

export const NAMETAGS: Record<string, string> = {
  // Native currency
  "0x0000000000000000000000000000000000000000": "ETH",

  // Users
  "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266": "Deployer",
  "0xaE9A0B4960FbAC50601E74064A2a89B326cEd73b": "Alice",

  // Contracts
  "0x5fbdb2315678afecb367f032d93f642f64180aa3": "USDC",
  "0xe7f1725e7734ce288f8367e1bb143e90bb3f0512": "DAI",
  "0x4200000000000000000000000000000000000042": "EnshrinedDEX",
};

export const TOKEN_DECIMALS: Record<string, number> = {
  "0x0000000000000000000000000000000000000000": 18, // ETH
  "0x5fbdb2315678afecb367f032d93f642f64180aa3": 6,  // USDC
  "0xe7f1725e7734ce288f8367e1bb143e90bb3f0512": 18, // DAI
};

export function getNameTag(address: string): string | undefined {
  // Normalize address to checksum format for lookup
  const normalized = address.toLowerCase();
  const key = Object.keys(NAMETAGS).find(k => k.toLowerCase() === normalized);
  return key ? NAMETAGS[key] : undefined;
}

export function getTokenDecimals(address: string): number {
  const normalized = address.toLowerCase();
  const key = Object.keys(TOKEN_DECIMALS).find(k => k.toLowerCase() === normalized);
  return key ? TOKEN_DECIMALS[key] : 18; // Default to 18 decimals
}

export function formatTokenAmount(amount: bigint, tokenAddress: string): string {
  const decimals = getTokenDecimals(tokenAddress);
  const divisor = BigInt(10 ** decimals);

  // Get whole and fractional parts
  const whole = amount / divisor;
  const remainder = amount % divisor;

  if (remainder === 0n) {
    return whole.toString();
  }

  // Format with up to 6 decimal places, removing trailing zeros
  const fractional = remainder.toString().padStart(decimals, '0');
  const trimmed = fractional.replace(/0+$/, '');

  if (trimmed === '') {
    return whole.toString();
  }

  return `${whole}.${trimmed}`;
}

export function formatAddress(address: string): string {
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
}
