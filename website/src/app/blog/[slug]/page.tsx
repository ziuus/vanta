import Link from "next/link";
import { ArrowLeft } from "lucide-react";

export default async function BlogPost({ params }: { params: Promise<{ slug: string }> }) {
  const { slug } = await params;
  
  return (
    <main className="min-h-screen pt-32 px-6 md:px-20 pb-20">
      <div className="max-w-3xl mx-auto">
        <Link href="/blog" className="inline-flex items-center gap-2 text-sm font-medium text-gray-500 hover:text-black transition-colors mb-12">
          <ArrowLeft size={16} />
          Back to Journal
        </Link>
        
        <p className="text-sm font-medium text-gray-400 mb-4">Sep 25, 2026</p>
        <h1 className="text-4xl md:text-6xl font-bold tracking-tight mb-8 capitalize leading-tight">
          {slug.replace(/-/g, " ")}
        </h1>
        
        <div className="prose prose-lg max-w-none text-gray-600 prose-headings:text-black">
          <p className="text-xl leading-relaxed mb-8">
            This is a placeholder for the actual blog post content. In a production environment, 
            this content would be fetched from a headless CMS like Sanity, Contentful, or statically generated from MDX files.
          </p>
          <h2 className="text-2xl font-semibold mb-4 mt-12 text-black">The Architecture</h2>
          <p className="mb-6">
            When building premium terminal experiences, you have to prioritize framerate and memory consumption above all else. 
            Vanta leverages the ratatui ecosystem to ensure UI elements are drawn directly to the terminal buffer without unnecessary intermediate representations.
          </p>
          <div className="liquid-glass p-8 my-10 border-l-4 border-l-black">
            <p className="italic text-black font-medium">
              "Performance isn't a feature; it's the foundation of everything we build."
            </p>
          </div>
          <p>
            By carefully managing the background thread execution and Extism WebAssembly sandboxing, we provide a rich plugin ecosystem that cannot accidentally freeze the main render loop.
          </p>
        </div>
      </div>
    </main>
  );
}
