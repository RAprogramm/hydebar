use freedesktop_icons::lookup;
use iced::widget::{image, svg};
use linicon_theme::get_icon_theme;
use log::{debug, trace};

use super::{TrayIcon, dbus::Icon};

pub(crate) fn icon_from_pixmaps(pixmaps: Vec<Icon,>,) -> Option<TrayIcon,>
{
    pixmaps
        .into_iter()
        .max_by_key(|icon| {
            trace!("tray icon w {}, h {}", icon.width, icon.height);
            (icon.width, icon.height,)
        },)
        .map(|mut icon| {
            for pixel in icon.bytes.chunks_exact_mut(4,) {
                pixel.rotate_left(1,);
            }

            TrayIcon::Image(image::Handle::from_rgba(
                icon.width as u32,
                icon.height as u32,
                icon.bytes,
            ),)
        },)
}

pub(crate) fn icon_from_name(icon_name: &str,) -> Option<TrayIcon,>
{
    debug!("resolving icon from name {icon_name}");

    let theme = get_icon_theme();
    if let Some(theme_name,) = &theme {
        debug!("icon theme found {theme_name}");
    }

    let icon_path = if let Some(theme_name,) = theme.as_deref() {
        // Try with theme first
        lookup(icon_name,)
            .with_cache()
            .with_theme(theme_name,)
            .find()
            // Fall back to default lookup if theme lookup fails
            .or_else(|| lookup(icon_name,).with_cache().find(),)
    } else {
        // No theme, use default lookup
        lookup(icon_name,).with_cache().find()
    }?;

    if icon_path.extension().is_some_and(|ext| ext == "svg",) {
        Some(TrayIcon::Svg(svg::Handle::from_path(icon_path,),),)
    } else {
        Some(TrayIcon::Image(image::Handle::from_path(icon_path,),),)
    }
}

#[cfg(test)]
fn icon_path_with_theme_fallback<F, G,>(
    theme: Option<String,>,
    mut themed_lookup: F,
    mut default_lookup: G,
) -> Option<PathBuf,>
where
    F: FnMut(&str,) -> Option<PathBuf,>,
    G: FnMut() -> Option<PathBuf,>,
{
    if let Some(theme_name,) = theme.as_deref()
        && let Some(path,) = themed_lookup(theme_name,)
    {
        return Some(path,);
    }

    default_lookup()
}

#[cfg(test)]
mod tests
{
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use super::icon_path_with_theme_fallback;

    #[test]
    fn uses_theme_when_available()
    {
        let theme_calls = AtomicUsize::new(0,);
        let default_calls = AtomicUsize::new(0,);

        let expected = PathBuf::from("/tmp/themed.svg",);

        let result = icon_path_with_theme_fallback(
            Some(String::from("test",),),
            |_| {
                theme_calls.fetch_add(1, Ordering::Relaxed,);
                Some(expected.clone(),)
            },
            || {
                default_calls.fetch_add(1, Ordering::Relaxed,);
                Some(PathBuf::from("/tmp/default.svg",),)
            },
        );

        assert_eq!(theme_calls.load(Ordering::Relaxed), 1);
        assert_eq!(default_calls.load(Ordering::Relaxed), 0);
        assert_eq!(result.as_deref(), Some(expected.as_path()));
    }

    #[test]
    fn falls_back_to_default_when_theme_missing()
    {
        let theme_calls = AtomicUsize::new(0,);
        let default_calls = AtomicUsize::new(0,);

        let expected = PathBuf::from("/tmp/default.svg",);

        let result = icon_path_with_theme_fallback(
            Some(String::from("test",),),
            |_| {
                theme_calls.fetch_add(1, Ordering::Relaxed,);
                None
            },
            || {
                default_calls.fetch_add(1, Ordering::Relaxed,);
                Some(expected.clone(),)
            },
        );

        assert_eq!(theme_calls.load(Ordering::Relaxed), 1);
        assert_eq!(default_calls.load(Ordering::Relaxed), 1);
        assert_eq!(result.as_deref(), Some(expected.as_path()));
    }
}
