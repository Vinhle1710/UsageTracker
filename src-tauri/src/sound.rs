#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sound {
    Default,
    None,
    Asterisk,
    Exclamation,
    Hand,
}
impl Sound {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Default" => Some(Self::Default),
            "None" => Some(Self::None),
            "Asterisk" => Some(Self::Asterisk),
            "Exclamation" => Some(Self::Exclamation),
            "Hand" => Some(Self::Hand),
            _ => None,
        }
    }
    pub fn play(self) {
        #[cfg(target_os = "windows")]
        {
            let alias: Option<&[u16]> = match self {
                Self::Default | Self::None => None,
                Self::Asterisk => Some(&[
                    83, 121, 115, 116, 101, 109, 65, 115, 116, 101, 114, 105, 115, 107, 0,
                ]),
                Self::Exclamation => Some(&[
                    83, 121, 115, 116, 101, 109, 69, 120, 99, 108, 97, 109, 97, 116, 105, 111, 110,
                    0,
                ]),
                Self::Hand => Some(&[83, 121, 115, 116, 101, 109, 72, 97, 110, 100, 0]),
            };
            if let Some(alias) = alias {
                unsafe {
                    use std::ptr::null_mut;
                    use windows_sys::Win32::Media::Audio::{
                        PlaySoundW, SND_ALIAS, SND_ASYNC, SND_NODEFAULT,
                    };
                    PlaySoundW(
                        alias.as_ptr(),
                        null_mut(),
                        SND_ALIAS | SND_ASYNC | SND_NODEFAULT,
                    );
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn allowlist_is_exact() {
        assert!(Sound::parse("Asterisk").is_some());
        assert!(Sound::parse("C:\\evil.wav").is_none());
        assert!(Sound::parse("SystemAsterisk").is_none());
        assert_eq!(Sound::parse("None"), Some(Sound::None));
    }
}
