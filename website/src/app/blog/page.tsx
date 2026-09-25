import Link from "next/link";
import { ArrowRight } from "lucide-react";

const posts = [
  {
    title: "Vanta v0.10.38 Released: Performance Unleashed",
    date: "Sep 25, 2026",
    excerpt: "The latest release brings custom performance profiles, letting Vanta idle at under 4% CPU usage while maintaining a 60fps render loop when active.",
    slug: "vanta-v0-10-38-released"
  },
  {
    title: "Building Micro-Extensions with Extism",
    date: "Sep 20, 2026",
    excerpt: "How we re-architected Vanta to support completely sandboxed WebAssembly plugins, enabling an ecosystem of custom widgets.",
    slug: "building-micro-extensions"
  },
  {
    title: "Why We Chose Rust and Ratatui",
    date: "Sep 15, 2026",
    excerpt: "Terminal interfaces don't have to be ugly. Read our deep dive into the rendering architecture that powers Vanta's modern aesthetic.",
    slug: "why-rust-and-ratatui"
  }
];

export default function Blog() {
  return (
    <main className="min-h-screen pt-32 px-6 md:px-20">
      <div className="max-w-4xl mx-auto">
        <h1 className="text-5xl md:text-7xl font-bold tracking-tighter mb-16">Journal.</h1>
        
        <div className="flex flex-col gap-12">
          {posts.map((post, i) => (
            <article key={i} className="group cursor-pointer border-b border-black/10 pb-12">
              <Link href={`/blog/${post.slug}`} className="block">
                <p className="text-sm font-medium text-gray-400 mb-3">{post.date}</p>
                <div className="flex justify-between items-center mb-4">
                  <h2 className="text-3xl font-semibold group-hover:text-gray-600 transition-colors">{post.title}</h2>
                  <div className="w-10 h-10 rounded-full border border-black/10 flex items-center justify-center opacity-0 -translate-x-4 group-hover:opacity-100 group-hover:translate-x-0 transition-all duration-300">
                    <ArrowRight size={16} />
                  </div>
                </div>
                <p className="text-lg text-gray-500 max-w-2xl">{post.excerpt}</p>
              </Link>
            </article>
          ))}
        </div>
      </div>
    </main>
  );
}
