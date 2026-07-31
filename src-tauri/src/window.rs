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
}
