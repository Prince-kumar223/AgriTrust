import { Search } from 'lucide-react';
import { useMemo, useState } from 'react';

import { Button } from '../components/Button';
import { Card } from '../components/Card';
import { Skeleton } from '../components/Skeleton';
import { useCreateOffer, useListings, useTrades } from '../hooks/useAgriTrustData';

export function BuyerDashboardPage() {
  const [cropFilter, setCropFilter] = useState('');
  const [locationFilter, setLocationFilter] = useState('');
  const listings = useListings();
  const trades = useTrades();
  const createOffer = useCreateOffer();

  const filteredListings = useMemo(() => {
    return listings.data?.filter((listing) => {
      const cropMatch = listing.crop_type.toLowerCase().includes(cropFilter.toLowerCase());
      const locationMatch = (listing.farmer.location ?? '')
        .toLowerCase()
        .includes(locationFilter.toLowerCase());
      return cropMatch && locationMatch;
    });
  }, [cropFilter, listings.data, locationFilter]);

  return (
    <main className="mx-auto grid max-w-7xl gap-6 px-4 py-8 lg:grid-cols-[1.15fr_0.85fr]">
      <section>
        <h1 className="text-3xl font-bold text-[#1B4332]">Buyer dashboard</h1>
        <Card className="mt-6">
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="flex items-center gap-2 rounded-md border border-[#DAD2BE] bg-white px-3 py-2">
              <Search size={16} className="text-[#6B705C]" />
              <input
                className="w-full outline-none"
                placeholder="Filter by crop"
                value={cropFilter}
                onChange={(event) => setCropFilter(event.target.value)}
              />
            </label>
            <input
              className="rounded-md border border-[#DAD2BE] px-3 py-2 outline-none focus:border-[#1B4332]"
              placeholder="Filter by farmer location"
              value={locationFilter}
              onChange={(event) => setLocationFilter(event.target.value)}
            />
          </div>
        </Card>

        <div className="mt-6 grid gap-4">
          {listings.isLoading ? <Skeleton className="h-40 w-full" /> : null}
          {listings.isError ? (
            <Card>
              <p className="text-sm text-red-700">Listings could not load.</p>
              <Button className="mt-3" onClick={() => listings.refetch()}>
                Retry
              </Button>
            </Card>
          ) : null}
          {filteredListings?.map((listing) => (
            <Card key={listing.id}>
              <div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
                <div>
                  <h2 className="text-xl font-semibold text-[#1B4332]">{listing.crop_type}</h2>
                  <p className="mt-1 text-sm text-[#5F695D]">
                    {listing.quantity} {listing.unit} · {listing.price_per_unit} per {listing.unit}
                  </p>
                  <p className="mt-2 text-sm leading-6 text-[#5F695D]">
                    {listing.description || 'No description provided.'}
                  </p>
                </div>
                <form
                  className="grid min-w-56 gap-2"
                  onSubmit={(event) => {
                    event.preventDefault();
                    const form = new FormData(event.currentTarget);
                    createOffer.mutate({
                      listing: listing.id,
                      offered_price: String(form.get('offered_price') ?? ''),
                      message: String(form.get('message') ?? ''),
                    });
                    event.currentTarget.reset();
                  }}
                >
                  <input
                    required
                    name="offered_price"
                    className="rounded-md border border-[#DAD2BE] px-3 py-2 outline-none focus:border-[#1B4332]"
                    placeholder="Offer price"
                  />
                  <input
                    name="message"
                    className="rounded-md border border-[#DAD2BE] px-3 py-2 outline-none focus:border-[#1B4332]"
                    placeholder="Message"
                  />
                  <Button isLoading={createOffer.isPending}>Make Offer</Button>
                </form>
              </div>
            </Card>
          ))}
        </div>
      </section>

      <aside>
        <Card>
          <h2 className="font-semibold text-[#1B4332]">My trades</h2>
          {trades.isLoading ? <Skeleton className="mt-4 h-32 w-full" /> : null}
          <div className="mt-4 grid gap-3">
            {trades.data?.map((trade) => (
              <a
                key={trade.id}
                href={`/trades/${trade.id}`}
                className="rounded-md bg-[#F7F4EC] p-3 text-sm hover:bg-[#EFE7D3]"
              >
                {trade.on_chain_trade_id}
                <span className="ml-2 capitalize text-[#1B4332]">{trade.status}</span>
              </a>
            ))}
          </div>
        </Card>
      </aside>
    </main>
  );
}
