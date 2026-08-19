#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clamps_popover_to_work_area_above_tray_point() {
        assert_eq!(
            popover_position(
                Rect {
                    x: 0,
                    y: 0,
                    width: 1280,
                    height: 720
                },
                (320, 240),
                (1260, 700)
            ),
            (948, 448)
        );
    }
    #[test]
    fn detached_popover_ignores_focus_loss_and_restores_position() {
        let mut state = PopoverState::default();
        state.detach((400, 300));
        assert!(!state.should_hide_on_focus_loss());
        assert_eq!(state.persisted_position, Some((400, 300)));
        state.attach();
        assert!(state.should_hide_on_focus_loss());
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
pub fn popover_position(area: Rect, size: (i32, i32), tray: (i32, i32)) -> (i32, i32) {
    let margin = 12;
    let x = (tray.0 - size.0 + 8).clamp(area.x + margin, area.x + area.width - size.0 - margin);
    let y = (tray.1 - size.1 - 12).clamp(area.y + margin, area.y + area.height - size.1 - margin);
    (x, y)
}
#[derive(Default)]
pub struct PopoverState {
    pub detached: bool,
    pub persisted_position: Option<(i32, i32)>,
}
impl PopoverState {
    pub fn detach(&mut self, position: (i32, i32)) {
        self.detached = true;
        self.persisted_position = Some(position);
    }
    pub fn attach(&mut self) {
        self.detached = false;
    }
    pub fn should_hide_on_focus_loss(&self) -> bool {
        !self.detached
    }
}
