//! Sets the terminal to [`raw`] mode for the duration of the returned object's lifetime.

/// Sets the terminal to raw mode for the duration of the returned object's lifetime.
pub fn raw() -> impl Drop {
    Raw::default()
}
struct Raw();
impl Default for Raw {
    fn default() -> Self {
        std::thread::yield_now();
        crossterm::terminal::enable_raw_mode()
            .expect("should be able to transition into raw mode");
        Raw()
    }
}
impl Drop for Raw {
    fn drop(&mut self) {
        crossterm::terminal::disable_raw_mode()
            .expect("should be able to transition out of raw mode");
    }
}
