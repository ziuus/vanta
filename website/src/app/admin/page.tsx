"use client";

import { useState, useEffect } from "react";
import { auth, db } from "@/lib/firebase";
import { signInWithEmailAndPassword, onAuthStateChanged, signOut, User } from "firebase/auth";
import { collection, addDoc } from "firebase/firestore";

export default function AdminPanel() {
  const [user, setUser] = useState<User | null>(null);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);

  // Form states
  const [title, setTitle] = useState("");
  const [slug, setSlug] = useState("");
  const [date, setDate] = useState("");
  const [author, setAuthor] = useState("");
  const [excerpt, setExcerpt] = useState("");
  const [content, setContent] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [success, setSuccess] = useState(false);

  useEffect(() => {
    const unsubscribe = onAuthStateChanged(auth, (currentUser) => {
      setUser(currentUser);
      setLoading(false);
    });
    return () => unsubscribe();
  }, []);

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    try {
      await signInWithEmailAndPassword(auth, email, password);
    } catch (err: any) {
      setError("Invalid credentials or user not found.");
    }
  };

  const handlePublish = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    setError("");
    setSuccess(false);

    try {
      await addDoc(collection(db, "blogs"), {
        title,
        slug,
        date,
        author,
        excerpt,
        content
      });
      setSuccess(true);
      setTitle(""); setSlug(""); setDate(""); setAuthor(""); setExcerpt(""); setContent("");
    } catch (err: any) {
      setError("Failed to publish: " + err.message);
    } finally {
      setIsSubmitting(false);
    }
  };

  if (loading) {
    return <div className="min-h-screen bg-[var(--bg-base)] flex items-center justify-center text-[var(--text-primary)] font-mono animate-pulse">Initializing Secure Uplink...</div>;
  }

  if (!user) {
    return (
      <main className="min-h-screen bg-[var(--bg-base)] flex items-center justify-center p-6">
        <div className="w-full max-w-md bg-[var(--bg-surface)] border border-[var(--border-subtle)] p-8">
          <h1 className="text-2xl font-bold mb-6 text-[var(--text-primary)] font-mono">ADMIN_ACCESS</h1>
          {error && <div className="mb-4 text-red-500 font-mono text-sm">[{error}]</div>}
          <form onSubmit={handleLogin} className="flex flex-col gap-4">
            <input type="email" placeholder="Email" value={email} onChange={e => setEmail(e.target.value)} className="bg-[var(--bg-elevated)] border border-[var(--border-subtle)] text-[var(--text-primary)] p-3 outline-none focus:border-accent font-mono" required />
            <input type="password" placeholder="Password" value={password} onChange={e => setPassword(e.target.value)} className="bg-[var(--bg-elevated)] border border-[var(--border-subtle)] text-[var(--text-primary)] p-3 outline-none focus:border-accent font-mono" required />
            <button type="submit" className="mt-4 bg-[var(--text-primary)] text-[var(--bg-base)] font-bold py-3 uppercase tracking-widest hover:bg-accent transition-colors">Authenticate</button>
          </form>
        </div>
      </main>
    );
  }

  return (
    <main className="min-h-screen bg-[var(--bg-base)] p-6 md:p-12 text-[var(--text-primary)]">
      <div className="max-w-4xl mx-auto">
        <div className="flex justify-between items-center mb-12 border-b border-[var(--border-subtle)] pb-6">
          <h1 className="text-3xl font-black tracking-tighter">CONTENT_CONTROL</h1>
          <button onClick={() => signOut(auth)} className="font-mono text-sm text-[var(--text-secondary)] hover:text-red-500 transition-colors">Sign Out</button>
        </div>

        <form onSubmit={handlePublish} className="flex flex-col gap-6">
          {success && <div className="p-4 bg-green-500/10 border border-green-500 text-green-500 font-mono text-sm">[SUCCESS] Post published to production.</div>}
          {error && <div className="p-4 bg-red-500/10 border border-red-500 text-red-500 font-mono text-sm">[ERROR] {error}</div>}
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="flex flex-col gap-2">
              <label className="font-mono text-xs uppercase text-[var(--text-secondary)]">Title</label>
              <input type="text" value={title} onChange={e => setTitle(e.target.value)} className="bg-[var(--bg-surface)] border border-[var(--border-subtle)] p-3 outline-none focus:border-accent" required />
            </div>
            <div className="flex flex-col gap-2">
              <label className="font-mono text-xs uppercase text-[var(--text-secondary)]">URL Slug</label>
              <input type="text" value={slug} onChange={e => setSlug(e.target.value.toLowerCase().replace(/\s+/g, '-'))} placeholder="e.g. my-first-post" className="bg-[var(--bg-surface)] border border-[var(--border-subtle)] p-3 outline-none focus:border-accent font-mono text-sm" required />
            </div>
            <div className="flex flex-col gap-2">
              <label className="font-mono text-xs uppercase text-[var(--text-secondary)]">Date</label>
              <input type="date" value={date} onChange={e => setDate(e.target.value)} className="bg-[var(--bg-surface)] border border-[var(--border-subtle)] p-3 outline-none focus:border-accent font-mono" required />
            </div>
            <div className="flex flex-col gap-2">
              <label className="font-mono text-xs uppercase text-[var(--text-secondary)]">Author</label>
              <input type="text" value={author} onChange={e => setAuthor(e.target.value)} className="bg-[var(--bg-surface)] border border-[var(--border-subtle)] p-3 outline-none focus:border-accent" required />
            </div>
          </div>

          <div className="flex flex-col gap-2">
            <label className="font-mono text-xs uppercase text-[var(--text-secondary)]">Short Excerpt</label>
            <textarea value={excerpt} onChange={e => setExcerpt(e.target.value)} rows={2} className="bg-[var(--bg-surface)] border border-[var(--border-subtle)] p-3 outline-none focus:border-accent" required />
          </div>

          <div className="flex flex-col gap-2">
            <label className="font-mono text-xs uppercase text-[var(--text-secondary)]">HTML Content</label>
            <textarea value={content} onChange={e => setContent(e.target.value)} rows={12} className="bg-[var(--bg-surface)] border border-[var(--border-subtle)] p-3 outline-none focus:border-accent font-mono text-sm" required />
          </div>

          <button type="submit" disabled={isSubmitting} className="mt-6 bg-accent text-[var(--bg-base)] font-bold py-4 uppercase tracking-widest hover:opacity-80 transition-opacity disabled:opacity-50">
            {isSubmitting ? "Publishing..." : "Publish to Live"}
          </button>
        </form>
      </div>
    </main>
  );
}
