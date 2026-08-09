export type Role = 'farmer' | 'buyer';

export type User = {
  id: number;
  username: string;
  role: Role;
  wallet_address: string;
  phone?: string;
  location?: string;
};

export type CropListing = {
  id: number;
  farmer: User;
  crop_type: string;
  quantity: string;
  unit: string;
  price_per_unit: string;
  description: string;
  status: 'active' | 'offered' | 'sold' | 'closed';
  created_at: string;
};

export type Offer = {
  id: number;
  listing: number;
  buyer: User;
  offered_price: string;
  message: string;
  status: 'pending' | 'accepted' | 'rejected';
  created_at: string;
};

export type TradeStatus = 'pending' | 'funded' | 'delivered' | 'completed' | 'disputed';

export type Trade = {
  id: number;
  offer: number;
  on_chain_trade_id: string;
  contract_address: string;
  status: TradeStatus;
  create_tx_hash: string;
  deposit_tx_hash: string;
  delivery_tx_hash: string;
  confirmation_tx_hash: string;
  dispute_tx_hash: string;
  created_at: string;
  updated_at: string;
};

export type AnalyticsSummary = {
  total_trades: number;
  total_volume: string;
  completion_rate: number;
  trades_by_status: Record<string, number>;
};
