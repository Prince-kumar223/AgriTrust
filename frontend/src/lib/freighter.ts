import {
  getAddress,
  isAllowed,
  isConnected,
  requestAccess,
  signTransaction,
} from '@stellar/freighter-api';

export type WalletState =
  | { status: 'not_installed'; message: string }
  | { status: 'locked'; message: string }
  | { status: 'connected'; address: string };

export async function getWalletState(): Promise<WalletState> {
  const connected = await isConnected();
  if (!connected.isConnected) {
    return {
      status: 'not_installed',
      message: 'Freighter is not available in this browser.',
    };
  }

  const allowed = await isAllowed();
  if (!allowed.isAllowed) {
    return {
      status: 'locked',
      message: 'Freighter is installed but locked or not approved for AgriTrust.',
    };
  }

  const address = await getAddress();
  return { status: 'connected', address: address.address };
}

export async function connectFreighter(): Promise<string> {
  const access = await requestAccess();
  if (!access.address) {
    throw new Error('Wallet access was not approved. Please unlock Freighter and try again.');
  }
  return access.address;
}

export async function signEscrowTransaction(transactionXdr: string, networkPassphrase: string) {
  const signed = await signTransaction(transactionXdr, { networkPassphrase });
  return signed.signedTxXdr;
}
