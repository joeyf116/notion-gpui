use crate::{
    platform,
    services::{ai::ChatProvider, ai::DemoProvider, ai::OpenAiProvider, storage},
    shell::AppRoot,
    state,
};
use gpui_kit::*;
use std::{cell::RefCell, error::Error, io, rc::Rc, sync::Arc};

pub fn run() -> Result<(), Box<dyn Error>> {
    let path = platform::state_path()?;
    let loaded = storage::load(&path)?;
    let provider: Arc<dyn ChatProvider> = match OpenAiProvider::from_env() {
        Some(provider) => Arc::new(provider),
        None => Arc::new(DemoProvider),
    };
    let startup_error = Rc::new(RefCell::new(None));
    let window_error = startup_error.clone();

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            state::initialize(loaded, path, cx);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            let options = WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("Notion GPUI".into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::centered(size(px(1100.), px(760.)), cx)),
                window_min_size: Some(size(px(800.), px(600.))),
                ..Default::default()
            };
            if let Err(error) = gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| AppRoot::new(provider, window, cx))
            }) {
                *window_error.borrow_mut() = Some(format!("Unable to open the window: {error}"));
                cx.quit();
                return;
            }
            cx.activate(true);
        });
    if let Some(error) = startup_error.borrow_mut().take() {
        return Err(io::Error::other(error).into());
    }
    Ok(())
}
