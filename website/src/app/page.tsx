"use client";

import { useEffect, useRef } from "react";
import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import Lenis from "@studio-freight/lenis";
import { Terminal, Cpu, Braces, Command } from "lucide-react";

export default function Home() {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Smooth scroll setup
    const lenis = new Lenis({
      duration: 1.2,
      easing: (t) => Math.min(1, 1.001 - Math.pow(2, -10 * t)),
      orientation: "vertical",
      gestureOrientation: "vertical",
      smoothWheel: true,
      touchMultiplier: 2,
    });

    function raf(time: number) {
      lenis.raf(time);
      requestAnimationFrame(raf);
    }
    requestAnimationFrame(raf);

    // GSAP setup
    gsap.registerPlugin(ScrollTrigger);
    
    // Hero Text Stagger
    const heroChars = gsap.utils.toArray(".hero-char");
    gsap.fromTo(heroChars, 
      { y: 100, opacity: 0, rotateX: -90 },
      {
        y: 0,
        opacity: 1,
        rotateX: 0,
        stagger: 0.05,
        duration: 1.2,
        ease: "power4.out",
        delay: 0.2
      }
    );

    gsap.fromTo(".fade-in-up",
      { y: 40, opacity: 0 },
      { y: 0, opacity: 1, duration: 1, stagger: 0.1, ease: "power3.out", delay: 0.6 }
    );

    // Scroll parallax for terminal window
    gsap.to(".terminal-window", {
      y: -100,
      ease: "none",
      scrollTrigger: {
        trigger: ".hero-section",
        start: "top top",
        end: "bottom top",
        scrub: true
      }
    });

    // Horizontal scroll for abstract metric cards
    const horizontalScroll = gsap.utils.toArray(".h-scroll-card");
    gsap.fromTo(horizontalScroll,
      { x: 100, opacity: 0 },
      {
        x: 0,
        opacity: 1,
        stagger: 0.1,
        scrollTrigger: {
          trigger: ".metric-section",
          start: "top 80%",
        }
      }
    );

    return () => {
      lenis.destroy();
      ScrollTrigger.getAll().forEach(t => t.kill());
    };
  }, []);

  const heroText = "VANTA".split("");

  return (
    <main ref={containerRef} className="relative w-full overflow-hidden bg-[var(--bg-base)]">
      <div className="aurora-bg"></div>
      <div className="terminal-grid absolute inset-0 z-0 opacity-30"></div>
      
      {/* 1. HERO SECTION */}
      <section className="hero-section relative min-h-screen pt-40 px-6 md:px-12 flex flex-col z-10">
        
        <div className="w-full flex justify-between items-start">
          <h1 className="text-[clamp(5rem,15vw,18rem)] font-black tracking-tighter leading-[0.8] flex" style={{ perspective: 1000 }}>
            {heroText.map((char, i) => (
              <span key={i} className="hero-char inline-block origin-bottom">{char}</span>
            ))}
            <span className="text-accent hero-char inline-block origin-bottom">_</span>
          </h1>
          
          <div className="hidden md:block max-w-[200px] text-right mt-6 fade-in-up">
            <div className="text-xs font-mono text-[var(--text-secondary)] mb-2 uppercase tracking-widest border-b border-[var(--border-subtle)] pb-2">Status</div>
            <div className="text-sm font-mono text-accent">SYSTEM.ONLINE</div>
            <div className="text-sm font-mono text-[var(--text-secondary)]">V0.10.38 / RUST</div>
          </div>
        </div>

        <div className="mt-20 md:mt-auto mb-10 w-full flex flex-col md:flex-row justify-between items-end gap-10 fade-in-up">
          <p className="text-xl md:text-3xl max-w-2xl font-light leading-snug text-[var(--text-secondary)]">
            A highly-optimized, aesthetic terminal dashboard. 
            <span className="text-[var(--text-primary)] block mt-2">Zero electron. Zero bloat. Pure mechanical performance.</span>
          </p>

          <a href="https://github.com/ziuus/vanta" className="group flex items-center gap-4 bg-[var(--text-primary)] text-[var(--bg-base)] px-8 py-5 rounded-none font-bold tracking-widest uppercase hover:bg-accent transition-colors duration-300">
            <Terminal size={20} className="group-hover:animate-pulse" />
            Initialize
          </a>
        </div>
      </section>

      {/* 2. THE TERMINAL WINDOW (Parallax Midground) */}
      <section className="relative px-6 md:px-12 pb-32 z-20">
        <div className="terminal-window liquid-glass w-full aspect-video md:aspect-[21/9] flex flex-col relative overflow-hidden group">
          <div className="h-10 border-b border-[var(--border-subtle)] flex items-center px-4 gap-2 bg-[var(--bg-elevated)]">
            <div className="w-3 h-3 rounded-full bg-red-500/80"></div>
            <div className="w-3 h-3 rounded-full bg-yellow-500/80"></div>
            <div className="w-3 h-3 rounded-full bg-green-500/80"></div>
            <div className="ml-4 font-mono text-xs text-[var(--text-secondary)] opacity-50 flex-1 text-center pr-12">vanta ~ release --locked</div>
          </div>
          <div className="flex-1 p-6 font-mono text-sm text-[var(--text-secondary)] relative z-10">
            <div className="text-accent mb-2">$ vanta start</div>
            <div>[INFO] Loading Extism Sandbox...</div>
            <div>[INFO] Initializing UI thread... 60FPS lock acquired.</div>
            <div>[INFO] Memory footprint: 12MB.</div>
            <div className="mt-4 animate-pulse">_</div>
          </div>
          
          {/* Abstract glow inside terminal */}
          <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[80%] h-[80%] bg-accent/5 rounded-full blur-[100px] group-hover:bg-accent/10 transition-colors duration-1000 pointer-events-none"></div>
        </div>
      </section>

      {/* 3. ASYMMETRICAL STATS / FEATURES */}
      <section className="metric-section relative px-6 md:px-12 py-32 border-t border-[var(--border-subtle)]">
        <div className="grid grid-cols-1 md:grid-cols-12 gap-6">
          <div className="md:col-span-4 h-scroll-card border border-[var(--border-subtle)] bg-[var(--bg-surface)] p-8">
            <Cpu className="text-[var(--text-secondary)] mb-12" size={24} />
            <div className="text-5xl font-black mb-4">4%</div>
            <p className="text-[var(--text-secondary)] font-mono text-sm uppercase">Idle CPU Footprint</p>
          </div>
          
          <div className="md:col-span-8 h-scroll-card border border-[var(--border-subtle)] bg-[var(--bg-surface)] p-8 flex flex-col justify-between relative overflow-hidden group">
            <Braces className="text-[var(--text-secondary)] mb-12 relative z-10" size={24} />
            <div className="relative z-10">
              <h3 className="text-3xl font-semibold mb-4">Extism WASM Plugins</h3>
              <p className="text-[var(--text-secondary)] text-lg max-w-xl">Compile isolated components in Rust, Go, or AssemblyScript. Sandboxed micro-extensions guarantee Vanta never crashes.</p>
            </div>
            {/* Massive background typography */}
            <div className="absolute -bottom-10 -right-10 text-[10rem] font-black text-[var(--bg-elevated)] leading-none select-none group-hover:scale-110 group-hover:rotate-6 transition-transform duration-700 pointer-events-none">WASM</div>
          </div>
          
          <div className="md:col-span-5 md:col-start-8 h-scroll-card liquid-glass p-8 mt-10 md:-mt-20 relative z-20">
            <Command className="text-accent mb-6" size={24} />
            <h3 className="text-2xl font-semibold mb-2 text-[var(--text-primary)]">Vim-style keybindings</h3>
            <p className="text-[var(--text-secondary)]">Navigate effortlessly. Lowercase for panels, Shift for global actions. Your hands never leave the keyboard.</p>
          </div>
        </div>
      </section>

      {/* FOOTER */}
      <footer className="px-6 md:px-12 py-10 flex justify-between items-end border-t border-[var(--border-subtle)]">
        <div>
          <div className="font-black text-2xl tracking-tighter mb-2">VANTA</div>
          <p className="text-[var(--text-secondary)] text-sm font-mono uppercase">Open Source Terminal Interface</p>
        </div>
        <a href="https://github.com/ziuus/vanta" className="text-sm font-mono text-[var(--text-secondary)] hover:text-accent transition-colors">github.com/ziuus/vanta</a>
      </footer>
    </main>
  );
}
