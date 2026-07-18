// Register GSAP plugins
gsap.registerPlugin(ScrollTrigger);

// 1. Initial Load Animations
const tl = gsap.timeline();

tl.from(".nav", {
  y: -50,
  opacity: 0,
  duration: 0.8,
  ease: "power3.out"
})
.from(".fade-in", {
  y: 30,
  opacity: 0,
  duration: 0.8,
  stagger: 0.2,
  ease: "power3.out"
}, "-=0.4")
.from(".terminal-mockup", {
  x: 50,
  opacity: 0,
  rotateY: 0,
  duration: 1,
  ease: "power3.out"
}, "-=0.6");

// 2. Split Text Simulation (for Hero)
const heroTitle = document.querySelector('.split-text');
if (heroTitle) {
  // A simple simulated split-text for GSAP since we didn't include the paid SplitText plugin
  // We'll just animate the whole header with a slight scale/blur
  gsap.from(heroTitle, {
    scale: 0.95,
    opacity: 0,
    duration: 1.2,
    ease: "power3.out",
    delay: 0.2
  });
}

// 3. Scroll Reveal Animations (Bento Grid)
gsap.utils.toArray(".gs-reveal").forEach(function(elem) {
  ScrollTrigger.create({
    trigger: elem,
    start: "top 85%",
    onEnter: function() {
      gsap.fromTo(elem, 
        { y: 50, opacity: 0 },
        { y: 0, opacity: 1, duration: 0.8, ease: "power3.out" }
      );
    },
    once: true
  });
});

// 4. Scroll Reveal (Left / Right Diagonal Section)
gsap.utils.toArray(".gs-reveal-left").forEach(function(elem) {
  ScrollTrigger.create({
    trigger: elem,
    start: "top 80%",
    onEnter: function() {
      gsap.fromTo(elem, 
        { x: -50, opacity: 0 },
        { x: 0, opacity: 1, duration: 0.8, ease: "power3.out" }
      );
    },
    once: true
  });
});

gsap.utils.toArray(".gs-reveal-right").forEach(function(elem) {
  ScrollTrigger.create({
    trigger: elem,
    start: "top 80%",
    onEnter: function() {
      gsap.fromTo(elem, 
        { x: 50, opacity: 0 },
        { x: 0, opacity: 1, duration: 0.8, ease: "power3.out", stagger: 0.1 }
      );
    },
    once: true
  });
});

// 5. Magnetic Hover Effect
const magneticElements = document.querySelectorAll(".magnetic");

magneticElements.forEach((elem) => {
  elem.addEventListener("mousemove", (e) => {
    const rect = elem.getBoundingClientRect();
    const x = e.clientX - rect.left - rect.width / 2;
    const y = e.clientY - rect.top - rect.height / 2;
    
    gsap.to(elem, {
      x: x * 0.3,
      y: y * 0.3,
      duration: 0.4,
      ease: "power2.out"
    });
  });

  elem.addEventListener("mouseleave", () => {
    gsap.to(elem, {
      x: 0,
      y: 0,
      duration: 0.6,
      ease: "elastic.out(1, 0.3)"
    });
  });
});

// 6. Floating Animation for Terminal Mockup
gsap.to(".floating", {
  y: -15,
  rotationX: 8,
  rotationY: -10,
  duration: 4,
  repeat: -1,
  yoyo: true,
  ease: "sine.inOut"
});
