import { api } from './api';
import type { TradeStatus } from './types';

const key = 'agritrust.pendingTradeSync';

export type PendingTradeSync = {
  type: 'transition';
  tradeId: number;
  status: TradeStatus;
  tx_hash: string;
};

export type PendingCreateTradeSync = {
  type: 'create';
  offer: number;
  on_chain_trade_id: string;
  contract_address: string;
  create_tx_hash: string;
};

export type PendingSync = PendingTradeSync | PendingCreateTradeSync;

function readQueue(): PendingSync[] {
  try {
    return JSON.parse(localStorage.getItem(key) ?? '[]') as PendingSync[];
  } catch {
    return [];
  }
}

function writeQueue(queue: PendingSync[]) {
  localStorage.setItem(key, JSON.stringify(queue));
  window.dispatchEvent(new Event('agritrust-sync-queue'));
}

export function enqueueTradeSync(item: Omit<PendingTradeSync, 'type'>) {
  const queue = readQueue().filter(
    (queued) =>
      queued.type !== 'transition' ||
      queued.tradeId !== item.tradeId ||
      queued.status !== item.status,
  );
  writeQueue([...queue, { ...item, type: 'transition' }]);
}

export function enqueueCreateTradeSync(item: Omit<PendingCreateTradeSync, 'type'>) {
  const queue = readQueue().filter(
    (queued) => queued.type !== 'create' || queued.offer !== item.offer,
  );
  writeQueue([...queue, { ...item, type: 'create' }]);
}

export function pendingTradeSyncCount() {
  return readQueue().length;
}

export async function flushTradeSyncQueue() {
  const remaining: PendingSync[] = [];
  for (const item of readQueue()) {
    try {
      if (item.type === 'create') {
        await api.post('/trades/', {
          offer: item.offer,
          on_chain_trade_id: item.on_chain_trade_id,
          contract_address: item.contract_address,
          create_tx_hash: item.create_tx_hash,
        });
      } else {
        await api.post(`/trades/${item.tradeId}/sync_state/`, {
          status: item.status,
          tx_hash: item.tx_hash,
        });
      }
    } catch {
      remaining.push(item);
    }
  }
  writeQueue(remaining);
  return remaining.length;
}
