"use client";

import { useEffect, useRef } from "react";
import { motion, useScroll, useTransform } from "framer-motion";
import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import Lenis from "@studio-freight/lenis";
import { Terminal, Cpu, Zap, Box } from "lucide-react";

export default function Home() {
  const containerRef = useRef<HTMLDivElement>(null);
  const heroTextRef = useRef<HTMLHeadingElement>(null);
  const { scrollYProgress } = useScroll();
  const y = useTransform(scrollYProgress, [0, 1], [0, 200]);

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
    
    // Bento grid reveal
    const cards = gsap.utils.toArray(".bento-card");
    gsap.fromTo(cards, 
      { y: 100, opacity: 0 },
      {
        y: 0,
        opacity: 1,
        stagger: 0.1,
        duration: 1,
        ease: "power3.out",
        scrollTrigger: {
          trigger: ".bento-container",
          start: "top 80%",
        }
      }
    );

    return () => {
      lenis.destroy();
      ScrollTrigger.getAll().forEach(t => t.kill());
    };
  }, []);

  return (
    <main ref={containerRef} className="relative w-full overflow-hidden">
      
      {/* BACKGROUND MESH */}
      <div className="fixed inset-0 z-[-1] pointer-events-none opacity-40 mix-blend-multiply">
        <div className="absolute top-[-20%] left-[-10%] w-[50%] h-[50%] bg-blue-300 rounded-full blur-[120px]" />
        <div className="absolute bottom-[10%] right-[-10%] w-[60%] h-[60%] bg-purple-200 rounded-full blur-[150px]" />
        <div className="absolute top-[40%] left-[30%] w-[40%] h-[40%] bg-green-200 rounded-full blur-[100px]" />
      </div>

      {/* HERO SECTION */}
      <section className="relative min-h-[120vh] flex flex-col justify-center px-6 md:px-20 pt-32">
        <motion.div style={{ y }} className="max-w-6xl">
          <motion.div
            initial={{ opacity: 0, y: 30 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, ease: [0.16, 1, 0.3, 1] }}
          >
            <h1 ref={heroTextRef} className="text-[clamp(4rem,10vw,12rem)] font-bold leading-[0.9] tracking-tighter text-[#111]">
              TERMINAL <br/>
              <span className="text-gray-400">EVOLVED.</span>
            </h1>
          </motion.div>
          
          <motion.p 
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.4, duration: 1 }}
            className="mt-8 text-xl md:text-3xl max-w-2xl text-gray-600 font-light"
          >
            Vanta is a highly-optimized, aesthetic terminal dashboard written in Rust. 
            No electron. No bloat. Pure performance.
          </motion.p>
          
          <motion.div 
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.6, duration: 0.8 }}
            className="mt-12 flex gap-4"
          >
            <a href="https://github.com/ziuus/vanta" className="liquid-glass px-8 py-4 rounded-full font-medium hover:scale-105 transition-transform duration-300 inline-flex items-center gap-2">
              <Terminal size={20} />
              View Source
            </a>
            <a href="#features" className="px-8 py-4 rounded-full font-medium border border-black/10 hover:bg-black/5 transition-colors duration-300">
              Discover Features
            </a>
          </motion.div>
        </motion.div>
      </section>

      {/* BENTO GRID SECTION */}
      <section id="features" className="bento-container relative min-h-screen px-6 md:px-20 py-32 z-10">
        <div className="max-w-7xl mx-auto">
          <h2 className="text-4xl md:text-6xl font-bold mb-16 tracking-tight">The ultimate <br/>developer interface.</h2>
          
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6 auto-rows-[300px]">
            {/* Cell 1: Large feature */}
            <div className="bento-card md:col-span-2 liquid-glass p-10 flex flex-col justify-between group hover:border-black/20 transition-colors">
              <div>
                <Zap className="text-accent mb-6" size={32} />
                <h3 className="text-3xl font-semibold mb-2">Blazing Fast</h3>
                <p className="text-gray-500 text-lg max-w-md">Written in pure Rust with ratatui. Zero-cost abstractions and lock-free concurrency ensure it runs at a buttery 60fps.</p>
              </div>
              <div className="text-[120px] font-black tracking-tighter text-black/5 leading-none self-end mt-[-60px] group-hover:scale-110 transition-transform duration-700">60FPS</div>
            </div>
            
            {/* Cell 2: Square */}
            <div className="bento-card liquid-glass p-10 flex flex-col justify-between group hover:border-black/20 transition-colors">
              <div>
                <Cpu className="mb-6" size={32} />
                <h3 className="text-xl font-semibold mb-2">Zero Bloat</h3>
                <p className="text-gray-500">Idles at less than 4% CPU. Perfect for running 24/7 on a secondary monitor.</p>
              </div>
              <div className="w-full h-2 bg-black/10 rounded-full overflow-hidden mt-8">
                <motion.div 
                  initial={{ width: 0 }} 
                  whileInView={{ width: "4%" }} 
                  transition={{ duration: 1, delay: 0.5 }}
                  className="h-full bg-black rounded-full"
                />
              </div>
            </div>

            {/* Cell 3: Square */}
            <div className="bento-card liquid-glass p-10 flex flex-col justify-between group hover:border-black/20 transition-colors">
              <div>
                <Box className="mb-6" size={32} />
                <h3 className="text-xl font-semibold mb-2">WASM Plugins</h3>
                <p className="text-gray-500">Write custom plugins in any language. Sandboxed securely via Extism.</p>
              </div>
              <div className="text-xs font-mono text-gray-400 mt-8 bg-black/5 p-4 rounded-lg">
                cargo build --target wasm32-unknown-unknown
              </div>
            </div>

            {/* Cell 4: Rectangle */}
            <div className="bento-card md:col-span-2 liquid-glass p-10 flex flex-col justify-between group hover:border-black/20 transition-colors overflow-hidden relative">
              <div className="relative z-10">
                <h3 className="text-3xl font-semibold mb-2">Fully Configurable</h3>
                <p className="text-gray-500 text-lg max-w-md">Define your perfect layout with an intuitive TOML configuration file.</p>
              </div>
              {/* Decorative abstract shape */}
              <div className="absolute right-[-10%] bottom-[-50%] w-[300px] h-[300px] border border-black/10 rounded-full group-hover:scale-150 transition-transform duration-[1.5s] ease-out" />
              <div className="absolute right-[10%] bottom-[-20%] w-[150px] h-[150px] border border-black/10 rounded-full group-hover:scale-150 transition-transform duration-[1.5s] ease-out delay-75" />
            </div>
          </div>
        </div>
      </section>

      {/* FOOTER */}
      <footer className="py-20 text-center text-sm font-medium text-gray-400">
        <p>Built with precision.</p>
        <p className="mt-2">© 2026 Vanta Dashboard</p>
      </footer>
    </main>
  );
}
