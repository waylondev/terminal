use tracing::level_filters::LevelFilter;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::fmt;

use crate::error::Result;

/// Initialize logging with the specified debug level
pub fn init_logging(debug: bool) -> Result<()> {
    // Set log level based on debug flag
    let level = if debug {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };
    
    // Create a filter that applies the log level to all targets
    let targets = Targets::new().with_default(level);
    
    // Create the logging subscriber
    let subscriber = tracing_subscriber::registry()
        .with(fmt::layer()
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_level(true)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
        )
        .with(targets);
    
    // Initialize the subscriber
    subscriber.init();
    
    tracing::info!("Logging initialized with level: {:?}", level);
    Ok(())
}
