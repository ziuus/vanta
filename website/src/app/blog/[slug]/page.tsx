"use client";

import { useEffect, useState } from "react";
import { collection, query, where, getDocs } from "firebase/firestore";
import { db } from "@/lib/firebase";
import Link from "next/link";
import { ArrowLeft } from "lucide-react";
import { use } from "react";

interface BlogPost {
  id: string;
  title: string;
  content: string;
  date: string;
  author: string;
}

export default function BlogPost({ params }: { params: Promise<{ slug: string }> }) {
  const resolvedParams = use(params);
  const [post, setPost] = useState<BlogPost | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function fetchPost() {
      try {
        const q = query(collection(db, "blogs"), where("slug", "==", resolvedParams.slug));
        const snapshot = await getDocs(q);
        
        if (!snapshot.empty) {
          setPost({ id: snapshot.docs[0].id, ...snapshot.docs[0].data() } as BlogPost);
        }
      } catch (error) {
        console.error("Error fetching post:", error);
      } finally {
        setLoading(false);
      }
    }
    
    fetchPost();
  }, [resolvedParams.slug]);

  if (loading) {
    return (
      <main className="min-h-screen pt-40 px-6 md:px-20 pb-20 bg-[var(--bg-base)] flex items-center justify-center">
        <div className="text-[var(--text-secondary)] font-mono animate-pulse">Decrypting...</div>
      </main>
    );
  }

  if (!post) {
    return (
      <main className="min-h-screen pt-40 px-6 md:px-20 pb-20 bg-[var(--bg-base)] flex flex-col items-center justify-center">
        <div className="text-[var(--text-secondary)] font-mono mb-6">[ERROR] Entry not found.</div>
        <Link href="/blog" className="text-accent font-mono uppercase tracking-widest hover:underline">Return to index</Link>
      </main>
    );
  }

  return (
    <main className="min-h-screen pt-32 px-6 md:px-20 pb-20 bg-[var(--bg-base)]">
      <article className="max-w-3xl mx-auto">
        <Link href="/blog" className="inline-flex items-center gap-2 text-[var(--text-secondary)] hover:text-accent font-mono text-sm uppercase tracking-widest mb-12 transition-colors">
          <ArrowLeft size={16} /> Back
        </Link>
        
        <header className="mb-16">
          <div className="flex items-center gap-4 font-mono text-sm text-[var(--text-secondary)] mb-6">
            <span className="text-accent">{post.date}</span>
            <span>//</span>
            <span>{post.author}</span>
          </div>
          <h1 className="text-4xl md:text-6xl font-black tracking-tighter text-[var(--text-primary)] leading-tight">{post.title}</h1>
        </header>
        
        <div className="prose prose-invert prose-lg max-w-none prose-p:text-[var(--text-secondary)] prose-headings:text-[var(--text-primary)]">
          <div dangerouslySetInnerHTML={{ __html: post.content }} />
        </div>
      </article>
    </main>
  );
}
