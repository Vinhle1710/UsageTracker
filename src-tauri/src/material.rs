#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Material {
    Clear,
    Acrylic,
    Blur,
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardRegion {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub radius: i32,
}

pub fn material_for_theme(theme: &str) -> Material {
    match theme {
        "clear" => Material::Clear,
        "blur" => Material::Blur,
        "solid" => Material::Solid,
        _ => Material::Acrylic,
    }
}

pub fn parse_tint(color: &str, opacity: f32) -> Option<(u8, u8, u8, u8)> {
    let hex = color.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    Some((red, green, blue, alpha))
}

pub fn card_regions(
    size: (u32, u32),
    layout: &str,
    provider_count: usize,
    minimized: bool,
    scale: f32,
) -> Vec<CardRegion> {
    if minimized {
        return vec![CardRegion {
            x: 0,
            y: 0,
            width: size.0 as i32,
            height: size.1 as i32,
            radius: size.1 as i32,
        }];
    }

    let padding = (8.0 * scale).round() as i32;
    let gap = (9.0 * scale).round() as i32;
    let radius = (14.0 * scale).round() as i32;
    let width = size.0 as i32 - padding * 2;
    let height = size.1 as i32 - padding * 2;
    let count = provider_count.clamp(1, 2);
    if count == 1 {
        return vec![CardRegion {
            x: padding,
            y: padding,
            width,
            height,
            radius,
        }];
    }

    if layout == "provider-columns" {
        let available = width - gap;
        let first = (available + 1) / 2;
        vec![
            CardRegion {
                x: padding,
                y: padding,
                width: first,
                height,
                radius,
            },
            CardRegion {
                x: padding + first + gap,
                y: padding,
                width: available - first,
                height,
                radius,
            },
        ]
    } else {
        let available = height - gap;
        let first = (available + 1) / 2;
        vec![
            CardRegion {
                x: padding,
                y: padding,
                width,
                height: first,
                radius,
            },
            CardRegion {
                x: padding,
                y: padding + first + gap,
                width,
                height: available - first,
                radius,
            },
        ]
    }
}

#[cfg(target_os = "windows")]
pub fn apply_to_window(
    window: &tauri::WebviewWindow,
    material: Material,
    tint: (u8, u8, u8, u8),
    regions: &[CardRegion],
) -> Result<(), String> {
    use window_vibrancy::{apply_acrylic, apply_blur, clear_acrylic, clear_blur};
    use windows_sys::Win32::Graphics::Gdi::{
        CombineRgn, CreateRectRgn, CreateRoundRectRgn, DeleteObject, SetWindowRgn, RGN_OR,
    };

    let _ = clear_acrylic(window);
    let _ = clear_blur(window);
    match material {
        Material::Acrylic => {
            apply_acrylic(window, Some(tint)).map_err(|error| error.to_string())?
        }
        Material::Blur => apply_blur(window, Some(tint)).map_err(|error| error.to_string())?,
        Material::Clear | Material::Solid => {}
    }

    let hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
    unsafe {
        let combined = CreateRectRgn(0, 0, 0, 0);
        if combined.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        for region in regions {
            let card = CreateRoundRectRgn(
                region.x,
                region.y,
                region.x + region.width,
                region.y + region.height,
                region.radius * 2,
                region.radius * 2,
            );
            if card.is_null() {
                let _ = DeleteObject(combined);
                return Err(std::io::Error::last_os_error().to_string());
            }
            CombineRgn(combined, combined, card, RGN_OR);
            let _ = DeleteObject(card);
        }
        if SetWindowRgn(hwnd, combined, 1) == 0 {
            let _ = DeleteObject(combined);
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_supported_materials_without_guessing() {
        assert_eq!(material_for_theme("clear"), Material::Clear);
        assert_eq!(material_for_theme("acrylic"), Material::Acrylic);
        assert_eq!(material_for_theme("blur"), Material::Blur);
        assert_eq!(material_for_theme("solid"), Material::Solid);
        assert_eq!(material_for_theme("unknown"), Material::Acrylic);
    }

    #[test]
    fn parses_card_color_and_clamps_alpha() {
        assert_eq!(parse_tint("#07101f", 0.84), Some((7, 16, 31, 214)));
        assert_eq!(parse_tint("#ffffff", 2.0), Some((255, 255, 255, 255)));
        assert_eq!(parse_tint("navy", 0.8), None);
    }

    #[test]
    fn shapes_vertical_cards_without_covering_the_gap() {
        assert_eq!(
            card_regions((326, 360), "stacked-compact", 2, false, 1.0),
            vec![
                CardRegion {
                    x: 8,
                    y: 8,
                    width: 310,
                    height: 168,
                    radius: 14
                },
                CardRegion {
                    x: 8,
                    y: 185,
                    width: 310,
                    height: 167,
                    radius: 14
                },
            ]
        );
    }

    #[test]
    fn shapes_horizontal_cards_and_minimized_pill() {
        assert_eq!(
            card_regions((620, 184), "provider-columns", 2, false, 1.0).len(),
            2
        );
        assert_eq!(
            card_regions((36, 20), "stacked-compact", 2, true, 1.0),
            vec![CardRegion {
                x: 0,
                y: 0,
                width: 36,
                height: 20,
                radius: 20
            }]
        );
    }
}
