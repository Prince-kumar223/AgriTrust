import type { ReactNode } from 'react';

export function Card({ children, className = '' }: { children: ReactNode; className?: string }) {
  return (
    <section className={`rounded-lg border border-[#DAD2BE] bg-white p-4 shadow-sm ${className}`}>
      {children}
    </section>
  );
}
