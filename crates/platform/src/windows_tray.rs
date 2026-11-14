//! What the Windows tray icon needs beyond Tauri's: its frames as pixels, and whether to animate.
use windows::Win32::UI::WindowsAndMessaging::{
    SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
};

/// An 8-bit RGBA PNG as its pixels, width and height; none for any other kind.
pub fn rgba(png_bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buf.truncate(info.buffer_size());
    Some((buf, info.width, info.height))
}

/// Windows' *Show animations* setting (Accessibility → Visual effects); on when unreadable.
pub fn animations_enabled() -> bool {
    let mut on = windows::core::BOOL(1);
    // SAFETY: the call writes one BOOL into `on`, which outlives it.
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&mut on as *mut windows::core::BOOL).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    read.is_err() || on.as_bool()
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_tray_frame_decodes_to_its_pixels() {
        let png = include_bytes!("../../../app/icons/tray/32/idle-0.png");
        let (pixels, width, height) = super::rgba(png).expect("an RGBA PNG");
        assert_eq!((width, height, pixels.len()), (32, 32, 32 * 32 * 4));
        assert!(super::rgba(b"not a png").is_none());
        let _ = super::animations_enabled();
    }
}
