export interface SurfaceScroller {
  raf(time: number): void;
  scrollTo(target: number, options?: { immediate?: boolean }): void;
  destroy(): void;
}

export interface SurfaceMotionAdapters {
  createScroller(container: HTMLElement): SurfaceScroller;
  addTicker(callback: (time: number) => void): void;
  removeTicker(callback: (time: number) => void): void;
  animateEntrance(root: HTMLElement): () => void;
  animatePanel(panel: HTMLElement, direction: -1 | 1): void;
}

function reducedMotion(): boolean {
  return typeof window !== "undefined" && typeof window.matchMedia === "function"
    ? window.matchMedia("(prefers-reduced-motion: reduce)").matches
    : false;
}

const defaultAdapters: SurfaceMotionAdapters = {
  createScroller: (container) => new Lenis({
    wrapper: container,
    eventsTarget: container,
    autoRaf: false,
    smoothWheel: true,
    // Light damping on purpose: heavier smoothing reads as lag on a desktop
    // pointer, where the OS scroll it is replacing is instant.
    lerp: 0.1,
    wheelMultiplier: 0.9,
    allowNestedScroll: true,
    respectReducedMotion: true,
  }),
  addTicker: (callback) => gsap.ticker.add(callback),
  removeTicker: (callback) => gsap.ticker.remove(callback),
  animateEntrance: (root) => {
    const context = gsap.context(() => {
      const items = gsap.utils.toArray<HTMLElement>(".surface-motion-item");
      if (reducedMotion()) {
        gsap.set(items, { clearProps: "all" });
        return;
      }
      // Opacity-led and short: an 8px rise reads as settling into place, where a
      // larger travel reads as an effect. Matches the --dur-slow / --ease pair the
      // CSS transitions use, so JS and CSS motion feel like one system.
      gsap.fromTo(items, {
        autoAlpha: 0,
        y: 8,
      }, {
        autoAlpha: 1,
        y: 0,
        duration: 0.44,
        stagger: 0.03,
        ease: "power3.out",
        clearProps: "transform,visibility",
      });
      gsap.utils.toArray<SVGPathElement>("[data-history-line]").forEach((line) => {
        if (typeof line.getTotalLength !== "function") return;
        const length = line.getTotalLength();
        gsap.fromTo(line, { strokeDasharray: length, strokeDashoffset: length }, { strokeDashoffset: 0, duration: 0.7, ease: "power2.out" });
      });
    }, root);
    return () => context.revert();
  },
  animatePanel: (panel, direction) => {
    const items = [panel, ...panel.querySelectorAll<HTMLElement>(".settings-control-card, .settings-group, .runtime-health, .theme-option")];
    if (reducedMotion()) {
      gsap.set(items, { clearProps: "all" });
      return;
    }
    gsap.fromTo(items, {
      autoAlpha: 0,
      x: direction * 10,
    }, {
      autoAlpha: 1,
      x: 0,
      duration: 0.36,
      stagger: 0.028,
      ease: "power3.out",
      clearProps: "transform,visibility",
      overwrite: "auto",
    });
  },
};

export function enhanceSurface(
  root: HTMLElement,
  options: { adapters?: SurfaceMotionAdapters } = {},
): () => void {
  const adapters = options.adapters ?? defaultAdapters;
  const scrollContainer = root.querySelector<HTMLElement>("[data-smooth-scroll]");
  let scroller: SurfaceScroller | null = null;
  if (scrollContainer) {
    try {
      scroller = adapters.createScroller(scrollContainer);
    } catch {
      scroller = null;
    }
  }
  const tick = (time: number) => scroller?.raf(time * 1000);
  if (scrollContainer) scrollContainer.scrollTop = 0;
  if (scroller) {
    scroller.scrollTo(0, { immediate: true });
    adapters.addTicker(tick);
  }
  const clearEntrance = adapters.animateEntrance(root);

  const tabs = Array.from(root.querySelectorAll<HTMLButtonElement>('[role="tab"][data-nav-index]'));
  let activeIndex = Number(tabs.find((tab) => tab.getAttribute("aria-selected") === "true")?.dataset.navIndex ?? 0);
  const listeners = tabs.map((tab) => {
    const onClick = () => {
      const nextIndex = Number(tab.dataset.navIndex ?? activeIndex);
      const panelId = tab.getAttribute("aria-controls");
      const panel = panelId
        ? Array.from(root.querySelectorAll<HTMLElement>("[id]")).find((candidate) => candidate.id === panelId) ?? null
        : null;
      if (scrollContainer) scrollContainer.scrollTop = 0;
      scroller?.scrollTo(0, { immediate: true });
      if (panel) adapters.animatePanel(panel, nextIndex < activeIndex ? -1 : 1);
      activeIndex = nextIndex;
    };
    tab.addEventListener("click", onClick);
    return [tab, onClick] as const;
  });

  let disposed = false;
  return () => {
    if (disposed) return;
    disposed = true;
    listeners.forEach(([tab, listener]) => tab.removeEventListener("click", listener));
    clearEntrance();
    if (scroller) {
      adapters.removeTicker(tick);
      scroller.destroy();
    }
  };
}
import Lenis from "lenis";
import gsap from "gsap";
import "lenis/dist/lenis.css";
