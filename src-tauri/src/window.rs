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

pub fn friendly_monitor_label(index: usize, _id: &str, width: u32, height: u32) -> String {
    format!("Screen {} — {width}×{height}", index + 1)
}

pub fn overlay_size(
    layout: &str,
    scale: f32,
    provider_count: usize,
    minimized: bool,
) -> (u32, u32) {
    let (width, height) = if minimized {
        (36, 20)
    } else if layout == "provider-columns" {
        (520, 152)
    } else {
        let count = provider_count.clamp(1, 2) as u32;
        (286, 25 + count * 120 + (count - 1) * 5)
    };
    (
        (width as f32 * scale).round() as u32,
        (height as f32 * scale).round() as u32,
    )
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
            "Screen 1 — 2560×1440"
        );
    }
    #[test]
    fn stacked_size_fits_one_or_two_provider_cards() {
        assert_eq!(overlay_size("stacked-compact", 1.0, 1, false), (286, 145));
        assert_eq!(overlay_size("stacked-compact", 1.0, 2, false), (286, 270));
    }
    #[test]
    fn column_size_scales_and_minimize_is_small() {
        assert_eq!(overlay_size("provider-columns", 1.25, 2, false), (650, 190));
        assert_eq!(overlay_size("provider-columns", 1.5, 2, true), (54, 30));
    }
}
