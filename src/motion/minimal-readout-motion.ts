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
  const run = (forward: boolean): Promise<void> => new Promise((resolve) => {
    const event = forward ? "onComplete" : "onReverseComplete";
    let settled = false;
    const done = () => {
      if (settled) return;
      settled = true;
      timeline.eventCallback(event, null);
      resolve();
    };
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
    finish: (open) => { timeline.progress(open ? 1 : 0).pause(); },
    kill: () => timeline.kill(),
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
    const actions = Array.from(root.querySelectorAll<HTMLElement>(".minimal-readout__dock-action"));
    const timeline = gsap.timeline({ paused: true, defaults: { overwrite: "auto" } });
    timeline.fromTo(
      actions,
      { autoAlpha: 0, y: -8, scale: 0.82 },
      { autoAlpha: 1, y: 0, scale: 1, duration: 0.2, stagger: 0.035, ease: "back.out(1.35)" },
      0,
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
  for (const button of root.querySelectorAll<HTMLButtonElement>(".minimal-readout__dock-action")) {
    button.tabIndex = expanded ? 0 : -1;
    button.setAttribute("aria-hidden", String(!expanded));
    if (expanded) button.dataset.geometryVisible = "true";
    else delete button.dataset.geometryVisible;
  }
}

export function enhanceMinimalReadout(root: HTMLElement, options: EnhanceOptions): () => void {
  const adapters = options.adapters ?? defaultAdapters;
  const surface = root.querySelector<HTMLElement>(".minimal-readout__surface")!;
  const handle = root.querySelector<HTMLButtonElement>(".minimal-readout__dock-handle")!;
  const dock = root.querySelector<HTMLElement>(".minimal-readout__dock")!;
  const usageTimeline = adapters.createUsageTimeline(root);
  const dockTimeline = adapters.createDockTimeline(root);
  let usagePointer = false;
  let usageFocus = false;
  let dockPointer = false;
  let dockFocus = false;
  let usageDesired = false;
  let dockDesired = false;
  let usageExpanded = false;
  let dockExpanded = false;
  let usageQueue = Promise.resolve();
  let dockQueue = Promise.resolve();
  let disposed = false;

  root.dataset.usageExpanded = "false";
  root.dataset.dockExpanded = "false";

  const requestUsage = (expanded: boolean) => {
    usageDesired = expanded;
    usageQueue = usageQueue.then(async () => {
      if (disposed || usageExpanded === usageDesired) return;
      const target = usageDesired;
      if (target) {
        root.dataset.reserveUsage = "true";
        setWeeklyAccessibility(root, true);
        await options.onGeometryChange();
        if (disposed) return;
        if (adapters.reducedMotion()) usageTimeline.finish(true);
        else void usageTimeline.play();
        usageExpanded = true;
        root.dataset.usageExpanded = "true";
      } else {
        if (adapters.reducedMotion()) usageTimeline.finish(false);
        else await usageTimeline.reverse();
        if (disposed) return;
        usageExpanded = false;
        root.dataset.usageExpanded = "false";
        setWeeklyAccessibility(root, false);
        root.dataset.reserveUsage = "false";
        await options.onGeometryChange();
      }
      if (usageDesired !== target) requestUsage(usageDesired);
    });
  };

  const requestDock = (expanded: boolean) => {
    dockDesired = expanded;
    dockQueue = dockQueue.then(async () => {
      if (disposed || dockExpanded === dockDesired) return;
      const target = dockDesired;
      if (target) {
        root.dataset.reserveDock = "true";
        setDockAccessibility(root, true);
        await options.onGeometryChange();
        if (disposed) return;
        if (adapters.reducedMotion()) dockTimeline.finish(true);
        else void dockTimeline.play();
        dockExpanded = true;
        root.dataset.dockExpanded = "true";
      } else {
        if (adapters.reducedMotion()) dockTimeline.finish(false);
        else await dockTimeline.reverse();
        if (disposed) return;
        dockExpanded = false;
        root.dataset.dockExpanded = "false";
        setDockAccessibility(root, false);
        root.dataset.reserveDock = "false";
        await options.onGeometryChange();
      }
      if (dockDesired !== target) requestDock(dockDesired);
    });
  };

  const syncUsage = () => requestUsage(usagePointer || usageFocus);
  const syncDock = () => requestDock(dockPointer || dockFocus);
  const onUsageEnter = () => { usagePointer = true; syncUsage(); };
  const onUsageLeave = () => { usagePointer = false; syncUsage(); };
  const onUsageFocusIn = () => { usageFocus = true; syncUsage(); };
  const onUsageFocusOut = (event: FocusEvent) => {
    usageFocus = event.relatedTarget instanceof Node && surface.contains(event.relatedTarget);
    syncUsage();
  };
  const onDockEnter = () => { dockPointer = true; syncDock(); };
  const onDockLeave = (event: PointerEvent) => {
    const related = event.relatedTarget;
    dockPointer = related instanceof Node && (handle.contains(related) || dock.contains(related));
    syncDock();
  };
  const onDockFocusIn = () => { dockFocus = true; syncDock(); };
  const onDockFocusOut = (event: FocusEvent) => {
    const related = event.relatedTarget;
    dockFocus = related instanceof Node && (handle.contains(related) || dock.contains(related));
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
  for (const element of [handle, dock]) {
    element.addEventListener("pointerenter", onDockEnter);
    element.addEventListener("pointerleave", onDockLeave as EventListener);
    element.addEventListener("focusin", onDockFocusIn);
    element.addEventListener("focusout", onDockFocusOut as EventListener);
  }
  root.addEventListener("keydown", onKeyDown);

  return () => {
    if (disposed) return;
    disposed = true;
    surface.removeEventListener("pointerenter", onUsageEnter);
    surface.removeEventListener("pointerleave", onUsageLeave);
    surface.removeEventListener("focusin", onUsageFocusIn);
    surface.removeEventListener("focusout", onUsageFocusOut);
    for (const element of [handle, dock]) {
      element.removeEventListener("pointerenter", onDockEnter);
      element.removeEventListener("pointerleave", onDockLeave as EventListener);
      element.removeEventListener("focusin", onDockFocusIn);
      element.removeEventListener("focusout", onDockFocusOut as EventListener);
    }
    root.removeEventListener("keydown", onKeyDown);
    usageTimeline.kill();
    dockTimeline.kill();
  };
}
