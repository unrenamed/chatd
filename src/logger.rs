use log::SetLoggerError;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

lazy_static::lazy_static! {
    static ref LOG_ENCODER: Box<PatternEncoder> = Box::new(PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S)} | {({l}):5.5} | {f}:{L} — {m}{n}"));
}

#[cfg(not(tarpaulin_include))]
pub fn setup(output: Option<String>, level: log::LevelFilter) -> Result<(), SetLoggerError> {
    // Configure a console appender
    let console_appender = {
        let console = ConsoleAppender::builder()
            .target(Target::Stderr)
            .encoder(LOG_ENCODER.to_owned())
            .build();
        Appender::builder().build("console", Box::new(console))
    };

    // Configure a file appender if output is provided
    let file_appender = match output {
        Some(path) => {
            let logfile = FileAppender::builder()
                .encoder(LOG_ENCODER.to_owned())
                .build(path)
                .unwrap();
            Some(Appender::builder().build("logfile", Box::new(logfile)))
        }
        None => None,
    };

    // Build the logging configuration
    let mut config_builder = Config::builder().appender(console_appender);
    let mut root_builder = Root::builder().appender("console");

    // Add file appender to configuration if provided
    if let Some(appender) = file_appender {
        config_builder = config_builder.appender(appender);
        root_builder = root_builder.appender("logfile");
    }

    let config = config_builder.build(root_builder.build(level)).unwrap();

    // Initialize the logger with the configuration
    log4rs::init_config(config)?;

    Ok(())
}
