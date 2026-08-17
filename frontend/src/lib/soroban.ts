import { contract } from '@stellar/stellar-sdk';
import { signTransaction } from '@stellar/freighter-api';

import type { Offer, TradeStatus } from './types';

const networkPassphrase =
  import.meta.env.VITE_STELLAR_NETWORK_PASSPHRASE ?? 'Test SDF Network ; September 2015';
const rpcUrl = import.meta.env.VITE_STELLAR_RPC_URL ?? 'https://soroban-testnet.stellar.org';
const contractId = import.meta.env.VITE_AGRITRUST_CONTRACT_ID ?? '';
const assetContractId = import.meta.env.VITE_ESCROW_ASSET_CONTRACT_ID ?? '';

type TxResult = {
  txHash: string;
};

type AgriTrustContract = {
  create_trade: (
    args: {
      trade_id: string;
      farmer: string;
      buyer: string;
      asset: string;
      crop_details: string;
      price: bigint;
    },
    options?: { timeoutInSeconds?: number },
  ) => Promise<{ signAndSend: () => Promise<unknown> }>;
  deposit_payment: (
    args: { trade_id: string; buyer: string },
    options?: { timeoutInSeconds?: number },
  ) => Promise<{ signAndSend: () => Promise<unknown> }>;
  confirm_delivery: (
    args: { trade_id: string; farmer: string },
    options?: { timeoutInSeconds?: number },
  ) => Promise<{ signAndSend: () => Promise<unknown> }>;
  confirm_receipt: (
    args: { trade_id: string; buyer: string },
    options?: { timeoutInSeconds?: number },
  ) => Promise<{ signAndSend: () => Promise<unknown> }>;
};

function requireChainConfig() {
  if (!contractId || !assetContractId) {
    throw new Error('Soroban contract configuration is missing. Check frontend .env values.');
  }
}

async function client(publicKey: string) {
  requireChainConfig();
  return contract.Client.from<AgriTrustContract>({
    contractId,
    networkPassphrase,
    publicKey,
    rpcUrl,
    signTransaction,
  });
}

function txHash(sentTx: unknown): string {
  const response = sentTx as {
    sendTransactionResponse?: { hash?: string };
    getTransactionResponse?: { txHash?: string; hash?: string };
  };
  return (
    response.getTransactionResponse?.txHash ??
    response.getTransactionResponse?.hash ??
    response.sendTransactionResponse?.hash ??
    ''
  );
}

function toStroops(amount: string): bigint {
  const [whole, fractional = ''] = amount.split('.');
  return BigInt(`${whole}${fractional.padEnd(7, '0').slice(0, 7)}`);
}

export function buildOnChainTradeId(offerId: number) {
  return `agr_${offerId}_${Date.now()}`;
}

export async function createTradeOnChain(offer: Offer, tradeId: string): Promise<TxResult> {
  const farmer = offer.listing.farmer.wallet_address;
  const buyer = offer.buyer.wallet_address;
  const agriTrust = await client(farmer);
  const tx = await agriTrust.create_trade(
    {
      trade_id: tradeId,
      farmer,
      buyer,
      asset: assetContractId,
      crop_details: offer.listing.crop_type,
      price: toStroops(offer.offered_price),
    },
    { timeoutInSeconds: 60 },
  );
  const sent = await tx.signAndSend();
  return { txHash: txHash(sent) };
}

export async function advanceTradeOnChain(
  status: TradeStatus,
  tradeId: string,
  walletAddress: string,
): Promise<TxResult & { nextStatus: TradeStatus }> {
  const agriTrust = await client(walletAddress);
  const options = { timeoutInSeconds: 60 };
  if (status === 'pending') {
    const tx = await agriTrust.deposit_payment(
      { trade_id: tradeId, buyer: walletAddress },
      options,
    );
    return { txHash: txHash(await tx.signAndSend()), nextStatus: 'funded' };
  }
  if (status === 'funded') {
    const tx = await agriTrust.confirm_delivery(
      { trade_id: tradeId, farmer: walletAddress },
      options,
    );
    return { txHash: txHash(await tx.signAndSend()), nextStatus: 'delivered' };
  }
  if (status === 'delivered') {
    const tx = await agriTrust.confirm_receipt(
      { trade_id: tradeId, buyer: walletAddress },
      options,
    );
    return { txHash: txHash(await tx.signAndSend()), nextStatus: 'completed' };
  }
  throw new Error('This trade cannot advance from its current state.');
}
