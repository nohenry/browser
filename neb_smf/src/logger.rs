use std::{fmt::Arguments, sync::Arc};

use log::{Level, Record};
use tower_lsp::{lsp_types::MessageType, Client};

#[derive(Clone)]
pub struct ClientLogger(pub Arc<Client>);

impl ClientLogger {
    pub async fn log(&self, record: &Record<'_>) {
        let level = match record.level() {
            Level::Info => MessageType::INFO,
            Level::Warn => MessageType::WARNING,
            Level::Error => MessageType::ERROR,
            Level::Debug => MessageType::LOG,
            Level::Trace => MessageType::LOG,
        };
        self.0.log_message(level, record.args()).await
    }

    pub async fn info(&self, args: Arguments<'_>) {
        let record = Record::builder().args(args).level(Level::Info).build();
        self.log(&record).await
    }

    pub async fn error(&self, args: Arguments<'_>) {
        let record = Record::builder().args(args).level(Level::Error).build();
        self.log(&record).await
    }
}
