"use client";

import { useEffect, useState } from "react";
import { collection, getDocs, orderBy, query } from "firebase/firestore";
import { db } from "@/lib/firebase";
import Link from "next/link";
import { ArrowRight } from "lucide-react";

interface BlogPost {
  id: string;
  title: string;
  excerpt: string;
  date: string;
  slug: string;
}

export default function Blog() {
  const [posts, setPosts] = useState<BlogPost[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function fetchPosts() {
      try {
        const q = query(collection(db, "blogs"), orderBy("date", "desc"));
        const snapshot = await getDocs(q);
        const fetchedPosts = snapshot.docs.map(doc => ({
          id: doc.id,
          ...doc.data()
        })) as BlogPost[];
        setPosts(fetchedPosts);
      } catch (error) {
        console.error("Error fetching posts:", error);
      } finally {
        setLoading(false);
      }
    }
    
    fetchPosts();
  }, []);

  return (
    <main className="min-h-screen pt-40 px-6 md:px-20 pb-20 bg-[var(--bg-base)]">
      <div className="max-w-4xl mx-auto">
        <h1 className="text-5xl md:text-7xl font-black tracking-tighter mb-16 text-[var(--text-primary)]">
          ENGINEERING <br/><span className="text-[var(--text-secondary)]">JOURNAL.</span>
        </h1>

        {loading ? (
          <div className="text-[var(--text-secondary)] font-mono animate-pulse">Loading core logs...</div>
        ) : posts.length === 0 ? (
          <div className="text-[var(--text-secondary)] font-mono border border-[var(--border-subtle)] p-8">
            [INFO] No entries found in the database.
          </div>
        ) : (
          <div className="flex flex-col gap-8">
            {posts.map((post) => (
              <Link key={post.id} href={`/blog/${post.slug}`} className="group border border-[var(--border-subtle)] bg-[var(--bg-surface)] p-8 hover:border-[var(--text-primary)] transition-colors">
                <div className="text-xs font-mono text-accent mb-4">{post.date}</div>
                <h2 className="text-3xl font-bold mb-4 text-[var(--text-primary)]">{post.title}</h2>
                <p className="text-[var(--text-secondary)] text-lg mb-8">{post.excerpt}</p>
                <div className="flex items-center gap-2 font-mono text-sm uppercase tracking-widest text-[var(--text-primary)] group-hover:text-accent transition-colors">
                  Read Entry <ArrowRight size={16} />
                </div>
              </Link>
            ))}
          </div>
        )}
      </div>
    </main>
  );
}
