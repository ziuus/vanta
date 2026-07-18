gsap.registerPlugin(ScrollTrigger);

// 1. Lenis Smooth Scroll
const lenis = new Lenis({
  duration: 1.2,
  easing: (t) => Math.min(1, 1.001 - Math.pow(2, -10 * t)),
  direction: 'vertical',
  gestureDirection: 'vertical',
  smooth: true,
  mouseMultiplier: 1,
});
function raf(time) {
  lenis.raf(time);
  requestAnimationFrame(raf);
}
requestAnimationFrame(raf);

// 2. Custom Cursor
const cursorDot = document.querySelector('.cursor-dot');
const cursorRing = document.querySelector('.cursor-ring');
let mouseX = 0, mouseY = 0;
let ringX = 0, ringY = 0;

document.addEventListener('mousemove', (e) => {
  mouseX = e.clientX;
  mouseY = e.clientY;
  if(cursorDot) {
    cursorDot.style.left = `${mouseX}px`;
    cursorDot.style.top = `${mouseY}px`;
  }
});

gsap.ticker.add(() => {
  ringX += (mouseX - ringX) * 0.15;
  ringY += (mouseY - ringY) * 0.15;
  if(cursorRing) {
    cursorRing.style.left = `${ringX}px`;
    cursorRing.style.top = `${ringY}px`;
  }
});

const interactiveElements = document.querySelectorAll('a, button, .magnetic, .magnetic-text');
interactiveElements.forEach(el => {
  el.addEventListener('mouseenter', () => document.body.classList.add('cursor-hover'));
  el.addEventListener('mouseleave', () => document.body.classList.remove('cursor-hover'));
});

// 3. Magnetic Effect
const magneticElements = document.querySelectorAll(".magnetic");
magneticElements.forEach((elem) => {
  elem.addEventListener("mousemove", (e) => {
    const rect = elem.getBoundingClientRect();
    const x = e.clientX - rect.left - rect.width / 2;
    const y = e.clientY - rect.top - rect.height / 2;
    gsap.to(elem, { x: x * 0.3, y: y * 0.3, duration: 0.4, ease: "power2.out" });
  });
  elem.addEventListener("mouseleave", () => {
    gsap.to(elem, { x: 0, y: 0, duration: 0.6, ease: "elastic.out(1, 0.3)" });
  });
});

const magneticText = document.querySelectorAll(".magnetic-text");
magneticText.forEach((elem) => {
  elem.addEventListener("mousemove", (e) => {
    const rect = elem.getBoundingClientRect();
    const x = e.clientX - rect.left - rect.width / 2;
    const y = e.clientY - rect.top - rect.height / 2;
    gsap.to(elem, { x: x * 0.1, y: y * 0.1, duration: 0.4, ease: "power2.out" });
  });
  elem.addEventListener("mouseleave", () => {
    gsap.to(elem, { x: 0, y: 0, duration: 0.6, ease: "elastic.out(1, 0.3)" });
  });
});

// 4. Intro Sequence & Text Splitting
const introTl = gsap.timeline();
const splitWords = document.querySelectorAll('.split-words');
splitWords.forEach(el => {
  const words = el.innerText.split(' ');
  el.innerHTML = '';
  words.forEach(word => {
    const wrapper = document.createElement('span');
    wrapper.className = 'split-word';
    word.split('').forEach(char => {
      const charSpan = document.createElement('span');
      charSpan.className = 'split-char';
      charSpan.innerText = char;
      wrapper.appendChild(charSpan);
    });
    el.appendChild(wrapper);
    el.innerHTML += ' ';
  });
});

window.addEventListener('load', () => {
  introTl
    .to('.intro-logo', { clipPath: 'polygon(0% 0%, 100% 0%, 100% 100%, 0% 100%)', duration: 1, ease: "power4.out" })
    .to('.intro-progress-bar', { width: '100%', duration: 1, ease: "power2.inOut" }, "-=0.5")
    .to('.intro-overlay', { yPercent: -100, duration: 1, ease: "power4.inOut" }, "+=0.2")
    .from('.nav', { y: -50, opacity: 0, duration: 1, ease: "power4.out" }, "-=0.5")
    .from('.badge', { opacity: 0, y: 20, duration: 0.8 }, "-=0.8")
    .fromTo('.hero-title', { y: 50, opacity: 0, rotateX: -20 }, { y: 0, opacity: 1, rotateX: 0, duration: 1.2, ease: "power4.out" }, "-=0.8")
    .from('.hero-subtitle', { opacity: 0, y: 20, duration: 0.8 }, "-=0.9")
    .from('.hero-actions', { opacity: 0, y: 20, duration: 0.8 }, "-=0.9")
    .from('.terminal-mockup', { x: 100, opacity: 0, rotateY: 20, duration: 1.5, ease: "power4.out" }, "-=1");

  // Terminal Typing sequence
  const termTl = gsap.timeline({ delay: 2.5 });
  termTl
    .fromTo('.typing-text', { width: 0, display: 'inline-block', overflow: 'hidden', whiteSpace: 'nowrap' }, { width: '160px', duration: 1.5, ease: "steps(18)" })
    .to('.typing-text', { opacity: 0, duration: 0.1 }, "+=0.5")
    .to('.sys-loading', { opacity: 1, duration: 0.1 })
    .to('.sys-loading', { opacity: 0, duration: 0.1 }, "+=1")
    .to('.sys-dashboard', { opacity: 1, duration: 0.1 });
});

// 5. ScrollTrigger: Horizontal Scroll Section
if (document.querySelector('.showcase-section') && window.innerWidth > 1000) {
  const hTrack = document.querySelector('.horizontal-scroll-track');
  const hCards = gsap.utils.toArray('.h-card');
  // wait a bit for layout to settle
  setTimeout(() => {
    const totalScroll = hTrack.scrollWidth - window.innerWidth + 400;

    gsap.to(hCards, {
      xPercent: -100 * (hCards.length - 1),
      ease: "none",
      scrollTrigger: {
        trigger: ".showcase-section",
        pin: true,
        scrub: 1,
        start: "top 10%",
        end: () => `+=${totalScroll}`
      }
    });
  }, 500);
}

// 6. Scroll Reveal for Bento Grid and general elements
gsap.utils.toArray('.gs-reveal').forEach(elem => {
  ScrollTrigger.create({
    trigger: elem,
    start: "top 85%",
    onEnter: () => {
      gsap.fromTo(elem, 
        { y: 60, opacity: 0, scale: 0.98 },
        { y: 0, opacity: 1, scale: 1, duration: 1, ease: "power3.out" }
      );
    },
    once: true
  });
});

gsap.utils.toArray('.split-words').forEach(elem => {
  ScrollTrigger.create({
    trigger: elem,
    start: "top 85%",
    onEnter: () => {
      gsap.to(elem.querySelectorAll('.split-char'), {
        opacity: 1, y: 0, rotateX: 0, duration: 0.8, stagger: 0.02, ease: "power4.out"
      });
    },
    once: true
  });
});

// 7. Dynamic Chart Generation
const chartContainer = document.querySelector('.dynamic-chart');
if (chartContainer) {
  for(let i=0; i<20; i++) {
    const bar = document.createElement('div');
    bar.className = 'bar';
    bar.style.height = `${Math.random() * 100}%`;
    chartContainer.appendChild(bar);
  }
  setInterval(() => {
    const bars = chartContainer.querySelectorAll('.bar');
    bars.forEach(bar => {
      bar.style.height = `${Math.random() * 100}%`;
    });
  }, 1000);
}

// 8. 3D Terminal Mouse interaction
const terminal = document.querySelector('.terminal-mockup');
if (terminal && window.innerWidth > 1000) {
  document.addEventListener('mousemove', (e) => {
    const x = (e.clientX / window.innerWidth - 0.5) * 20;
    const y = (e.clientY / window.innerHeight - 0.5) * 20;
    gsap.to(terminal, {
      rotateY: -15 + x,
      rotateX: 10 - y,
      duration: 1,
      ease: "power2.out"
    });
  });
}

// 9. Three.js Particle Field Background
const canvas = document.getElementById('webgl-canvas');
if (canvas && typeof THREE !== 'undefined') {
  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 1000);
  const renderer = new THREE.WebGLRenderer({ canvas: canvas, alpha: true, antialias: true });
  renderer.setSize(window.innerWidth, window.innerHeight);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

  const particlesGeometry = new THREE.BufferGeometry();
  const particlesCount = 400;
  const posArray = new Float32Array(particlesCount * 3);

  for(let i = 0; i < particlesCount * 3; i++) {
    posArray[i] = (Math.random() - 0.5) * 20;
  }
  particlesGeometry.setAttribute('position', new THREE.BufferAttribute(posArray, 3));

  const material = new THREE.PointsMaterial({
    size: 0.03,
    color: 0xC084FC,
    transparent: true,
    opacity: 0.4,
    blending: THREE.AdditiveBlending
  });

  const particlesMesh = new THREE.Points(particlesGeometry, material);
  scene.add(particlesMesh);
  camera.position.z = 5;

  let mouseX3D = 0;
  let mouseY3D = 0;
  
  document.addEventListener('mousemove', (event) => {
    mouseX3D = (event.clientX / window.innerWidth) - 0.5;
    mouseY3D = (event.clientY / window.innerHeight) - 0.5;
  });

  const clock = new THREE.Clock();
  const tick = () => {
    const elapsedTime = clock.getElapsedTime();
    particlesMesh.rotation.y = elapsedTime * 0.05 + mouseX3D * 0.5;
    particlesMesh.rotation.x = elapsedTime * 0.02 + mouseY3D * 0.5;
    
    const positions = particlesGeometry.attributes.position.array;
    for(let i = 0; i < particlesCount; i++) {
      const i3 = i * 3;
      positions[i3 + 1] += Math.sin(elapsedTime + positions[i3]) * 0.002;
    }
    particlesGeometry.attributes.position.needsUpdate = true;

    renderer.render(scene, camera);
    window.requestAnimationFrame(tick);
  };
  tick();

  window.addEventListener('resize', () => {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
  });
}
