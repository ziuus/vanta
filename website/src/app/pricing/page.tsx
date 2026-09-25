import { Check } from "lucide-react";

export default function Pricing() {
  return (
    <main className="min-h-screen pt-32 px-6 md:px-20 pb-20">
      <div className="max-w-6xl mx-auto">
        <div className="text-center mb-20">
          <h1 className="text-5xl md:text-7xl font-bold tracking-tight mb-6">Simple, transparent pricing.</h1>
          <p className="text-xl text-gray-500 max-w-2xl mx-auto">
            Vanta is open-source and free forever for individuals. Upgrade for team collaboration, remote sync, and enterprise support.
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
          {/* Free Tier */}
          <div className="liquid-glass p-8 flex flex-col justify-between">
            <div>
              <h3 className="text-2xl font-semibold mb-2">Hacker</h3>
              <p className="text-gray-500 mb-6">For individuals and open-source.</p>
              <div className="text-5xl font-bold tracking-tight mb-8">$0<span className="text-lg text-gray-400 font-normal">/mo</span></div>
              
              <ul className="space-y-4 mb-8">
                <li className="flex items-center gap-3 text-gray-600"><Check size={18} className="text-black" /> Unlimited local configurations</li>
                <li className="flex items-center gap-3 text-gray-600"><Check size={18} className="text-black" /> All community WASM plugins</li>
                <li className="flex items-center gap-3 text-gray-600"><Check size={18} className="text-black" /> Extism sandbox</li>
              </ul>
            </div>
            <button className="w-full py-4 rounded-full border border-black/10 font-medium hover:bg-black/5 transition-colors">
              Get Started
            </button>
          </div>

          {/* Pro Tier */}
          <div className="liquid-glass p-8 flex flex-col justify-between relative transform md:-translate-y-4 shadow-xl border-black/20">
            <div className="absolute top-0 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-black text-white px-4 py-1 rounded-full text-sm font-medium">
              Most Popular
            </div>
            <div>
              <h3 className="text-2xl font-semibold mb-2">Pro</h3>
              <p className="text-gray-500 mb-6">For power users across devices.</p>
              <div className="text-5xl font-bold tracking-tight mb-8">$12<span className="text-lg text-gray-400 font-normal">/mo</span></div>
              
              <ul className="space-y-4 mb-8">
                <li className="flex items-center gap-3 text-gray-600"><Check size={18} className="text-black" /> Everything in Hacker</li>
                <li className="flex items-center gap-3 text-gray-600"><Check size={18} className="text-black" /> Config cloud sync</li>
                <li className="flex items-center gap-3 text-gray-600"><Check size={18} className="text-black" /> Private plugins registry</li>
                <li className="flex items-center gap-3 text-gray-600"><Check size={18} className="text-black" /> SSH integration proxy</li>
              </ul>
            </div>
            <button className="w-full py-4 rounded-full bg-black text-white font-medium hover:bg-black/90 transition-colors">
              Start 14-day Trial
            </button>
          </div>

          {/* Enterprise Tier */}
          <div className="liquid-glass p-8 flex flex-col justify-between">
            <div>
              <h3 className="text-2xl font-semibold mb-2">Enterprise</h3>
              <p className="text-gray-500 mb-6">For large infrastructure teams.</p>
              <div className="text-5xl font-bold tracking-tight mb-8">Custom</div>
              
              <ul className="space-y-4 mb-8">
                <li className="flex items-center gap-3 text-gray-600"><Check size={18} className="text-black" /> SSO & SAML</li>
                <li className="flex items-center gap-3 text-gray-600"><Check size={18} className="text-black" /> Custom AI integrations</li>
                <li className="flex items-center gap-3 text-gray-600"><Check size={18} className="text-black" /> SOC2 Compliance</li>
                <li className="flex items-center gap-3 text-gray-600"><Check size={18} className="text-black" /> Dedicated support</li>
              </ul>
            </div>
            <button className="w-full py-4 rounded-full border border-black/10 font-medium hover:bg-black/5 transition-colors">
              Contact Sales
            </button>
          </div>
        </div>
      </div>
    </main>
  );
}
