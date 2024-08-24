use fern::colors::{Color, ColoredLevelConfig};
use std::time::SystemTime;

pub fn setup_logger(log_path: &str) -> Result<(), fern::InitError> {
    let colors = ColoredLevelConfig::new()
        .info(Color::Magenta)
        .warn(Color::Yellow)
        .error(Color::Red);

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339_seconds(SystemTime::now()),
                colors.color(record.level()),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file(log_path)?)
        .apply()?;
    Ok(())
}
