fn main() {
    if let Err(error) = notion_gpui::app::run() {
        eprintln!("Unable to start Notion GPUI: {error}");
        std::process::exit(1);
    }
}
