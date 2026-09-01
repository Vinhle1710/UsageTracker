import gsap from "gsap";

export interface ReversibleTimeline {
  play(): Promise<void>;
  reverse(): Promise<void>;
  progress(): number;
  finish(open: boolean): void;
  kill(): void;
}

export interface MinimalMotionAdapters {
  createUsageTimeline(root: HTMLElement): ReversibleTimeline;
  createDockTimeline(root: HTMLElement): ReversibleTimeline;
  reducedMotion(): boolean;
}

interface EnhanceOptions {
  onGeometryChange: () => Promise<void>;
  adapters?: MinimalMotionAdapters;
}

function prefersReducedMotion(): boolean {
  return typeof window !== "undefined"
    && typeof window.matchMedia === "function"
    && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function wrapTimeline(timeline: gsap.core.Timeline): ReversibleTimeline {
  let activeDone: (() => void) | null = null;
  const settleActive = () => {
    const done = activeDone;
    activeDone = null;
    done?.();
  };
  const run = (forward: boolean): Promise<void> => new Promise((resolve) => {
    settleActive();
    const event = forward ? "onComplete" : "onReverseComplete";
    let settled = false;
    const done = () => {
      if (settled) return;
      settled = true;
      if (activeDone === done) activeDone = null;
      timeline.eventCallback(event, null);
      resolve();
    };
    activeDone = done;
    timeline.eventCallback(event, done);
    if (forward) timeline.play();
    else timeline.reverse();
    const progress = timeline.progress();
    if ((forward && progress === 1) || (!forward && progress === 0)) done();
  });
  return {
    play: () => run(true),
    reverse: () => run(false),
    progress: () => timeline.progress(),
    finish: (open) => { settleActive(); timeline.progress(open ? 1 : 0).pause(); },
    kill: () => { settleActive(); timeline.kill(); },
  };
}

export function dockBladeClip(edge: "left" | "right"): { from: string; to: string } {
  return {
    from: edge === "left" ? "inset(0 48% 0 0)" : "inset(0 0 0 48%)",
    to: "inset(0 0 0 0)",
  };
}

const defaultAdapters: MinimalMotionAdapters = {
  createUsageTimeline: (root) => {
    const surface = root.querySelector<HTMLElement>(".minimal-readout__surface")!;
    const providers = root.querySelector<HTMLElement>(".minimal-readout__providers")!;
    const weekly = Array.from(root.querySelectorAll<HTMLElement>(".minimal-readout__weekly"));
    const direction = root.dataset.edge === "left" ? -1 : 1;
    const timeline = gsap.timeline({ paused: true, defaults: { overwrite: "auto" } });
    timeline
      .to(surface, { width: "var(--minimal-expanded-width)", duration: 0.28, ease: "power3.out" }, 0)
      .to(providers, { x: 0, duration: 0.28, ease: "power3.out" }, 0)
      .fromTo(
        weekly,
        { autoAlpha: 0, x: direction * 8 },
        { autoAlpha: 1, x: 0, duration: 0.2, stagger: 0.035, ease: "power2.out" },
        0.06,
      );
    return wrapTimeline(timeline);
  },
  createDockTimeline: (root) => {
    const blade = root.querySelector<HTMLElement>(".minimal-readout__action-blade")!;
    const trigger = root.querySelector<HTMLElement>(".minimal-readout__dock-handle")!;
    const actions = Array.from(root.querySelectorAll<HTMLElement>(".minimal-readout__dock-action"));
    const clip = dockBladeClip(root.dataset.edge === "left" ? "left" : "right");
    const direction = root.dataset.edge === "left" ? -1 : 1;
    const timeline = gsap.timeline({ paused: true, defaults: { overwrite: "auto" } });
    timeline
      .fromTo(
        blade,
        { autoAlpha: 0, clipPath: clip.from },
        { autoAlpha: 1, clipPath: clip.to, duration: 0.22, ease: "power3.out" },
        0,
      )
      .to(trigger, { autoAlpha: 0, duration: 0.12, ease: "power2.out" }, 0)
      .fromTo(
        actions,
        { autoAlpha: 0, x: direction * 6, scale: 0.88 },
        { autoAlpha: 1, x: 0, scale: 1, duration: 0.16, stagger: 0.025, ease: "power2.out" },
        0.08,
      );
    return wrapTimeline(timeline);
  },
  reducedMotion: prefersReducedMotion,
};

function setWeeklyAccessibility(root: HTMLElement, expanded: boolean): void {
  for (const weekly of root.querySelectorAll<HTMLElement>(".minimal-readout__weekly")) {
    weekly.setAttribute("aria-hidden", String(!expanded));
  }
}

function setDockAccessibility(root: HTMLElement, expanded: boolean): void {
  const handle = root.querySelector<HTMLButtonElement>(".minimal-readout__dock-handle")!;
  const dock = root.querySelector<HTMLElement>(".minimal-readout__dock")!;
  handle.setAttribute("aria-expanded", String(expanded));
  dock.setAttribute("aria-hidden", String(!expanded));
  for (const button of root.querySelectorAll<HTMLButtonElement>(".minimal-readout__dock-action")) {
    button.tabIndex = expanded ? 0 : -1;
    button.setAttribute("aria-hidden", String(!expanded));
  }
}

export function enhanceMinimalReadout(root: HTMLElement, options: EnhanceOptions): () => void {
  const adapters = options.adapters ?? defaultAdapters;
  const surface = root.querySelector<HTMLElement>(".minimal-readout__surface")!;
  const actionShell = root.querySelector<HTMLElement>(".minimal-readout__action-shell")!;
  const usageTimeline = adapters.createUsageTimeline(root);
  const dockTimeline = adapters.createDockTimeline(root);
  let usagePointer = false;
  let usageFocus = false;
  let dockPointer = false;
  let dockFocus = false;
  let usageDesired = false;
  let dockDesired = false;
  let usageRevision = 0;
  let dockRevision = 0;
  let usageOpeningRevision = 0;
  let dockOpeningRevision = 0;
  let disposed = false;

  root.dataset.usageExpanded = "false";
  root.dataset.dockExpanded = "false";

  const requestUsage = (expanded: boolean) => {
    if (disposed || usageDesired === expanded) return;
    usageDesired = expanded;
    const revision = ++usageRevision;
    void (async () => {
      if (expanded) {
        usageOpeningRevision = revision;
        const alreadyReserved = root.dataset.reserveUsage === "true";
        root.dataset.reserveUsage = "true";
        setWeeklyAccessibility(root, true);
        if (!alreadyReserved) await options.onGeometryChange();
        if (disposed || revision !== usageRevision) {
          if (usageOpeningRevision === revision) usageOpeningRevision = 0;
          return;
        }
        if (adapters.reducedMotion()) usageTimeline.finish(true);
        else await usageTimeline.play();
        if (disposed || revision !== usageRevision) {
          if (usageOpeningRevision === revision) usageOpeningRevision = 0;
          return;
        }
        if (usageOpeningRevision === revision) usageOpeningRevision = 0;
        root.dataset.usageExpanded = "true";
      } else {
        if (adapters.reducedMotion()) usageTimeline.finish(false);
        else await usageTimeline.reverse();
        if (disposed || revision !== usageRevision) return;
        root.dataset.usageExpanded = "false";
        setWeeklyAccessibility(root, false);
        root.dataset.reserveUsage = "false";
        await options.onGeometryChange();
      }
    })();
  };

  const requestDock = (expanded: boolean) => {
    if (disposed || dockDesired === expanded) return;
    dockDesired = expanded;
    const revision = ++dockRevision;
    void (async () => {
      if (expanded) {
        dockOpeningRevision = revision;
        const alreadyReserved = root.dataset.reserveDock === "true";
        root.dataset.reserveDock = "true";
        setDockAccessibility(root, true);
        if (!alreadyReserved) await options.onGeometryChange();
        if (disposed || revision !== dockRevision) {
          if (dockOpeningRevision === revision) dockOpeningRevision = 0;
          return;
        }
        if (adapters.reducedMotion()) dockTimeline.finish(true);
        else await dockTimeline.play();
        if (disposed || revision !== dockRevision) {
          if (dockOpeningRevision === revision) dockOpeningRevision = 0;
          return;
        }
        if (dockOpeningRevision === revision) dockOpeningRevision = 0;
        root.dataset.dockExpanded = "true";
      } else {
        if (adapters.reducedMotion()) dockTimeline.finish(false);
        else await dockTimeline.reverse();
        if (disposed || revision !== dockRevision) return;
        root.dataset.dockExpanded = "false";
        setDockAccessibility(root, false);
        root.dataset.reserveDock = "false";
        await options.onGeometryChange();
      }
    })();
  };

  const syncUsage = () => requestUsage(usagePointer || usageFocus);
  const syncDock = () => requestDock(dockPointer || dockFocus);
  const onUsageEnter = () => {
    usagePointer = true;
    syncUsage();
  };
  const onUsageLeave = () => {
    usagePointer = false;
    syncUsage();
  };
  const onUsageFocusIn = () => {
    usageFocus = true;
    syncUsage();
  };
  const onUsageFocusOut = (event: FocusEvent) => {
    usageFocus = event.relatedTarget instanceof Node && surface.contains(event.relatedTarget);
    syncUsage();
  };
  const onDockEnter = () => {
    dockPointer = true;
    syncDock();
  };
  const onDockLeave = (event: PointerEvent) => {
    const related = event.relatedTarget;
    dockPointer = related instanceof Node && actionShell.contains(related);
    syncDock();
  };
  const onDockFocusIn = () => {
    dockFocus = true;
    syncDock();
  };
  const onDockFocusOut = (event: FocusEvent) => {
    const related = event.relatedTarget;
    dockFocus = related instanceof Node && actionShell.contains(related);
    syncDock();
  };
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key !== "Escape") return;
    usagePointer = false;
    usageFocus = false;
    dockPointer = false;
    dockFocus = false;
    requestUsage(false);
    requestDock(false);
  };

  surface.addEventListener("pointerenter", onUsageEnter);
  surface.addEventListener("pointerleave", onUsageLeave);
  surface.addEventListener("focusin", onUsageFocusIn);
  surface.addEventListener("focusout", onUsageFocusOut);
  actionShell.addEventListener("pointerenter", onDockEnter);
  actionShell.addEventListener("pointerleave", onDockLeave as EventListener);
  actionShell.addEventListener("focusin", onDockFocusIn);
  actionShell.addEventListener("focusout", onDockFocusOut as EventListener);
  root.addEventListener("keydown", onKeyDown);

  return () => {
    if (disposed) return;
    disposed = true;
    surface.removeEventListener("pointerenter", onUsageEnter);
    surface.removeEventListener("pointerleave", onUsageLeave);
    surface.removeEventListener("focusin", onUsageFocusIn);
    surface.removeEventListener("focusout", onUsageFocusOut);
    actionShell.removeEventListener("pointerenter", onDockEnter);
    actionShell.removeEventListener("pointerleave", onDockLeave as EventListener);
    actionShell.removeEventListener("focusin", onDockFocusIn);
    actionShell.removeEventListener("focusout", onDockFocusOut as EventListener);
    root.removeEventListener("keydown", onKeyDown);
    usageTimeline.kill();
    dockTimeline.kill();
  };
}
