use log::SetLoggerError;
use log4rs::{
    append::{
        console::{ConsoleAppender, Target},
        file::FileAppender,
    },
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};

static LOG_PATTERN: &'static str = "{d(%Y-%m-%d %H:%M:%S)} | {({l}):5.5} | {f}:{L} — {m}{n}";

pub fn setup(output: Option<String>, level: log::LevelFilter) -> Result<(), SetLoggerError> {
    // Configure a console appender
    let console_appender = {
        let console = ConsoleAppender::builder()
            .target(Target::Stderr)
            .encoder(Box::new(PatternEncoder::new(LOG_PATTERN)))
            .build();
        Appender::builder().build("console", Box::new(console))
    };

    // Configure a file appender if output is provided
    let file_appender = match output {
        Some(path) => {
            let logfile = FileAppender::builder()
                .encoder(Box::new(PatternEncoder::new(LOG_PATTERN)))
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
