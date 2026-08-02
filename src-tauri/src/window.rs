#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorInfo {
    pub id: String,
    pub area: Rect,
}
pub const MARGIN: i32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceRepairPlan {
    pub immediate: bool,
    pub deferred: bool,
    pub restore_cached_main_region: bool,
}

pub fn focus_surface_repair_plan(label: &str, _focused: bool) -> Option<SurfaceRepairPlan> {
    match label {
        "main" | "settings" => Some(SurfaceRepairPlan {
            immediate: true,
            deferred: true,
            restore_cached_main_region: false,
        }),
        _ => None,
    }
}

pub fn work_area_rect(
    _monitor_position: (i32, i32),
    _monitor_size: (u32, u32),
    work_position: (i32, i32),
    work_size: (u32, u32),
) -> Rect {
    Rect {
        x: work_position.0,
        y: work_position.1,
        w: work_size.0,
        h: work_size.1,
    }
}

pub fn friendly_monitor_label(index: usize, _id: &str, width: u32, height: u32) -> String {
    format!("Monitor {} — {width}×{height}", index + 1)
}

pub fn overlay_size(
    layout: &str,
    scale: f32,
    expanded_provider_count: usize,
    bubble_count: usize,
) -> (u32, u32) {
    let mixed_height = if bubble_count > 0 { 49 } else { 0 };
    let (width, height) = if expanded_provider_count == 0 {
        let count = bubble_count.min(2) as u32;
        if count == 0 {
            return (0, 0);
        }
        (count * 48 + count.saturating_sub(1) * 8, 48)
    } else if layout == "provider-columns" {
        (620, 184 + mixed_height)
    } else {
        let count = expanded_provider_count.clamp(1, 2) as u32;
        (326, 190 + (count - 1) * 170 + mixed_height)
    };
    (
        (width as f32 * scale).round() as u32,
        (height as f32 * scale).round() as u32,
    )
}

/// Card widths are a layout constant, so the measured DOM width is only ever allowed to grow
/// the overlay. Without this floor a restore is unrecoverable: the window is still bubble-sized
/// when the card is measured, the squeezed card reports a tiny width, and that width becomes the
/// next window width. Heights stay measured because a card's content genuinely varies in height.
pub fn resolve_overlay_width(
    layout_width: u32,
    measured_width: Option<u32>,
    expanded_provider_count: usize,
) -> u32 {
    let Some(measured) = measured_width else {
        return layout_width;
    };
    if expanded_provider_count == 0 {
        return measured;
    }
    measured.max(layout_width)
}

pub fn corner_position(area: Rect, size: (u32, u32), corner: &str) -> (i32, i32) {
    let (w, h) = (size.0 as i32, size.1 as i32);
    let (left, top) = (area.x + MARGIN, area.y + MARGIN);
    let right = area.x + area.w as i32 - w - MARGIN;
    let bottom = area.y + area.h as i32 - h - MARGIN;
    match corner {
        "top-left" => (left, top),
        "top-right" => (right, top),
        "bottom-left" => (left, bottom),
        _ => (right, bottom),
    }
}
pub fn choose_monitor<'a>(
    monitors: &'a [MonitorInfo],
    preferred_id: Option<&str>,
) -> Option<&'a MonitorInfo> {
    preferred_id
        .and_then(|id| monitors.iter().find(|m| m.id == id))
        .or_else(|| monitors.first())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_gain_requests_immediate_and_deferred_repair_for_main_and_settings() {
        let expected = SurfaceRepairPlan {
            immediate: true,
            deferred: true,
            restore_cached_main_region: false,
        };

        assert_eq!(focus_surface_repair_plan("main", true), Some(expected));
        assert_eq!(
            focus_surface_repair_plan("settings", true),
            Some(SurfaceRepairPlan {
                restore_cached_main_region: false,
                ..expected
            })
        );
    }

    #[test]
    fn focus_loss_requests_immediate_and_deferred_repair_for_main_and_settings() {
        let expected = SurfaceRepairPlan {
            immediate: true,
            deferred: true,
            restore_cached_main_region: false,
        };

        assert_eq!(focus_surface_repair_plan("main", false), Some(expected));
        assert_eq!(
            focus_surface_repair_plan("settings", false),
            Some(SurfaceRepairPlan {
                restore_cached_main_region: false,
                ..expected
            })
        );
    }

    #[test]
    fn unrelated_windows_do_not_request_surface_repair() {
        assert_eq!(focus_surface_repair_plan("other", true), None);
    }

    fn area() -> Rect {
        Rect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1080,
        }
    }
    fn monitors() -> Vec<MonitorInfo> {
        vec![
            MonitorInfo {
                id: "DISPLAY1".into(),
                area: area(),
            },
            MonitorInfo {
                id: "DISPLAY2".into(),
                area: Rect {
                    x: 1920,
                    y: 0,
                    w: 1920,
                    h: 1080,
                },
            },
        ]
    }
    #[test]
    fn bottom_right_default() {
        assert_eq!(
            corner_position(area(), (380, 380), "bottom-right"),
            (1528, 688)
        );
    }
    #[test]
    fn top_left() {
        assert_eq!(corner_position(area(), (380, 380), "top-left"), (12, 12));
    }
    #[test]
    fn respects_monitor_offset() {
        assert_eq!(
            corner_position(
                Rect {
                    x: 1920,
                    y: 0,
                    w: 1920,
                    h: 1080
                },
                (380, 380),
                "top-left"
            ),
            (1932, 12)
        );
    }
    #[test]
    fn picks_preferred_monitor() {
        assert_eq!(
            choose_monitor(&monitors(), Some("DISPLAY2")).unwrap().id,
            "DISPLAY2"
        );
    }
    #[test]
    fn falls_back_when_preferred_missing() {
        assert_eq!(
            choose_monitor(&monitors()[..1], Some("DISPLAY2"))
                .unwrap()
                .id,
            "DISPLAY1"
        );
    }
    #[test]
    fn returns_to_preferred_monitor() {
        assert_eq!(
            choose_monitor(&monitors(), Some("DISPLAY2")).unwrap().id,
            "DISPLAY2"
        );
    }
    #[test]
    fn no_monitors_yields_none() {
        assert!(choose_monitor(&[], Some("DISPLAY1")).is_none());
    }
    #[test]
    fn labels_monitor_without_exposing_raw_id() {
        assert_eq!(
            friendly_monitor_label(0, "DISPLAY2", 2560, 1440),
            "Monitor 1 — 2560×1440"
        );
    }
    #[test]
    fn stacked_size_fits_one_or_two_provider_cards() {
        assert_eq!(overlay_size("stacked-compact", 1.0, 1, 0), (326, 190));
        assert_eq!(overlay_size("stacked-compact", 1.0, 2, 0), (326, 360));
    }
    #[test]
    fn column_size_scales_and_bubble_only_sizes_are_exact() {
        assert_eq!(overlay_size("provider-columns", 1.25, 2, 0), (775, 230));
        assert_eq!(overlay_size("stacked-compact", 1.0, 0, 1), (48, 48));
        assert_eq!(overlay_size("provider-columns", 1.0, 0, 2), (104, 48));
        assert_eq!(overlay_size("stacked-compact", 1.0, 1, 1), (326, 239));
        assert_eq!(overlay_size("provider-columns", 1.0, 1, 1), (620, 233));
    }
    #[test]
    fn a_measured_width_never_collapses_an_expanded_layout_below_its_layout_width() {
        // Restoring a card while the window is still bubble-sized measures a card that the
        // narrow window squeezed. The layout width is the floor so the card can grow back.
        assert_eq!(resolve_overlay_width(326, Some(48), 1), 326);
        assert_eq!(resolve_overlay_width(620, Some(60), 2), 620);
    }

    #[test]
    fn a_measured_width_is_respected_when_it_exceeds_the_layout_width() {
        assert_eq!(resolve_overlay_width(620, Some(700), 2), 700);
    }

    #[test]
    fn a_bubble_only_row_keeps_its_measured_width_without_a_card_floor() {
        assert_eq!(resolve_overlay_width(48, Some(48), 0), 48);
        assert_eq!(resolve_overlay_width(104, Some(104), 0), 104);
    }

    #[test]
    fn an_unmeasured_width_falls_back_to_the_layout_width() {
        assert_eq!(resolve_overlay_width(326, None, 1), 326);
    }

    #[test]
    fn work_area_rect_excludes_taskbar_space() {
        assert_eq!(
            work_area_rect((0, 0), (1920, 1080), (0, 0), (1920, 1040)),
            Rect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1040,
            }
        );
        assert_eq!(
            corner_position(
                work_area_rect((0, 0), (1920, 1080), (0, 0), (1920, 1040)),
                (326, 360),
                "bottom-right"
            ),
            (1582, 668)
        );
    }
}
