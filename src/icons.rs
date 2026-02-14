use gtk4::Image;
use gtk4::gdk::Display;

pub fn load_app_icon(icon_name: &str, size: i32) -> Image {
    let theme = gtk4::IconTheme::for_display(&Display::default().expect("No display"));

    let icon = theme.lookup_icon(
        icon_name,
        &[],
        size,
        1,
        gtk4::TextDirection::Ltr,
        gtk4::IconLookupFlags::empty(),
    );
    let image = Image::from_paintable(Some(&icon));
    image.set_pixel_size(size);
    image
}
