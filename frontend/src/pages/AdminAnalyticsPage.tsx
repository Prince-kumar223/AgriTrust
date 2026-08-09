import {
  Cell,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  Bar,
  BarChart,
  XAxis,
  YAxis,
} from 'recharts';

import { Card } from '../components/Card';
import { Skeleton } from '../components/Skeleton';
import { useAnalytics } from '../hooks/useAgriTrustData';

const colors = ['#1B4332', '#D4A017', '#6B705C', '#B08968', '#40916C'];

export function AdminAnalyticsPage() {
  const analytics = useAnalytics();
  const statusRows = Object.entries(analytics.data?.trades_by_status ?? {}).map(
    ([status, count]) => ({
      status,
      count,
    }),
  );

  return (
    <main className="mx-auto max-w-7xl px-4 py-8">
      <h1 className="text-3xl font-bold text-[#1B4332]">Analytics</h1>
      {analytics.isLoading ? <Skeleton className="mt-6 h-72 w-full" /> : null}
      {analytics.isError ? (
        <Card className="mt-6 text-red-700">
          Analytics could not load. Retry after checking your admin session.
        </Card>
      ) : null}
      {analytics.data ? (
        <>
          <section className="mt-6 grid gap-4 md:grid-cols-3">
            <Metric label="Total trades" value={analytics.data.total_trades} />
            <Metric label="Total volume" value={analytics.data.total_volume} />
            <Metric
              label="Completion rate"
              value={`${Math.round(analytics.data.completion_rate * 100)}%`}
            />
          </section>
          <section className="mt-6 grid gap-4 lg:grid-cols-2">
            <Card>
              <h2 className="font-semibold text-[#1B4332]">Trades over time</h2>
              <div className="mt-4 h-72">
                <ResponsiveContainer>
                  <BarChart data={statusRows}>
                    <XAxis dataKey="status" />
                    <YAxis allowDecimals={false} />
                    <Tooltip />
                    <Bar dataKey="count" fill="#1B4332" radius={[6, 6, 0, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              </div>
            </Card>
            <Card>
              <h2 className="font-semibold text-[#1B4332]">Completion mix</h2>
              <div className="mt-4 h-72">
                <ResponsiveContainer>
                  <PieChart>
                    <Pie data={statusRows} dataKey="count" nameKey="status" outerRadius={100}>
                      {statusRows.map((row, index) => (
                        <Cell key={row.status} fill={colors[index % colors.length]} />
                      ))}
                    </Pie>
                    <Tooltip />
                  </PieChart>
                </ResponsiveContainer>
              </div>
            </Card>
          </section>
        </>
      ) : null}
    </main>
  );
}

function Metric({ label, value }: { label: string; value: string | number }) {
  return (
    <Card>
      <p className="text-sm text-[#5F695D]">{label}</p>
      <strong className="mt-2 block text-3xl text-[#1B4332]">{value}</strong>
    </Card>
  );
}
