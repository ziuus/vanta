import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Vanta | The Ultimate Terminal Dashboard",
  description: "Highly-optimized, aesthetic terminal dashboard written in Rust.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="antialiased font-sans">
        {/* Noise overlay for texture */}
        <div 
          className="fixed inset-0 z-[9999] pointer-events-none opacity-[0.02]"
          style={{
            backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)'/%3E%3C/svg%3E")`,
            backgroundRepeat: "repeat",
            backgroundSize: "256px 256px"
          }}
        />
        
        {/* Navigation */}
        <nav className="fixed top-0 left-0 right-0 z-50 p-6 flex justify-between items-center">
          <div className="font-bold text-2xl tracking-tighter mix-blend-difference">VANTA</div>
          <div className="flex gap-6 text-sm font-medium mix-blend-difference">
            <a href="/" className="hover:opacity-60 transition-opacity">Overview</a>
            <a href="/pricing" className="hover:opacity-60 transition-opacity">Pricing</a>
            <a href="/blog" className="hover:opacity-60 transition-opacity">Blog</a>
            <a href="https://github.com/ziuus/vanta" className="hover:opacity-60 transition-opacity">GitHub</a>
          </div>
        </nav>

        {children}
      </body>
    </html>
  );
}
