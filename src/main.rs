#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod client;
mod error;
mod page_parser;
mod settings;
mod storage;
mod workers;

use std::time::Duration;

use chrono::{Datelike, NaiveTime, TimeDelta, Timelike};

use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use native_dialog::{DialogBuilder, MessageLevel};

use crate::settings::{SMTPConfig, Settings};
use crate::workers::Workers;

pub struct TimeTable(Box<[NaiveTime]>);
impl TimeTable {
    pub fn new<Iter>(time_table: Iter) -> Self
    where
        Iter: IntoIterator<Item = NaiveTime>,
    {
        let mut time_table = time_table.into_iter().collect::<Vec<_>>();
        time_table.sort_unstable();
        time_table.dedup();
        Self(time_table.into_boxed_slice())
    }
}
impl Iterator for TimeTable {
    type Item = ();
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.len() == 0 {
            return None;
        }
        let now = chrono::Local::now().naive_local().time();
        let idx = match self.0.binary_search(&now) {
            Ok(_) => return Some(()),
            Err(idx) => idx % self.0.len(),
        };
        let mut delta = self.0[idx] - now;
        if delta.num_seconds() < 0 {
            delta += TimeDelta::days(1)
        }
        let duration = delta.to_std().unwrap_or(Duration::ZERO);
        std::thread::sleep(duration);
        Some(())
    }
}

fn show_alert(text: String, level: MessageLevel) {
    let _ = DialogBuilder::message()
        .set_level(level)
        .set_title("Tender Parser")
        .set_text(text)
        .alert()
        .show();
}

fn send_mails<I>(html_content: &str, mails: I, config: &SMTPConfig) -> Result<(), String>
where
    I: IntoIterator<Item = Mailbox>,
{
    let creds = Credentials::new(config.username.to_string(), config.password.to_string());

    let mailer = SmtpTransport::relay(config.relay.as_ref())
        .unwrap()
        .credentials(creds)
        .build();

    for mail_box in mails.into_iter() {
        let message = Message::builder()
            .from(config.username.clone())
            .to(mail_box)
            .subject("Тендеры")
            .header(lettre::message::header::ContentType::TEXT_HTML)
            .body(html_content.to_string())
            .map_err(|e| e.to_string())?;

        let response = mailer.send(&message).map_err(|e| e.to_string())?;
        if !response.is_positive() {
            let code = response.code();
            return Err(format!("Send mail error, response code: {code}"));
        }
    }
    Ok(())
}

fn panic_hook() {
    std::panic::set_hook(Box::new(|info| match info.payload_as_str() {
        Some(msg) => show_alert(format!("Fatal Error:\n'{msg}'"), MessageLevel::Error),
        None => show_alert("Fatal Error".into(), MessageLevel::Error),
    }));
}

fn init_main_loop() {
    let settings = Settings::load().unwrap_or_else(|e| panic!("{e}"));
    if settings.show_info {
        show_alert(
            "Parser has been launched".into(), 
            MessageLevel::Info
        );
    }
    let time_table = settings.time_table.clone();
    for _ in TimeTable::new(time_table) {
        if settings.show_info {
            show_alert("Run pursing session".into(), MessageLevel::Info);
        }
        let worker = Workers::new(settings.keywords.clone(), settings.pages_limit);
        
        let parsing_res = worker.join().unwrap_or_else(|e| panic!("{e}"));
        match parsing_res {
            Err(e) => {
                show_alert(e.to_string(), MessageLevel::Warning);
                continue;
            }
            Ok(parsing_res) => {
                let html = parsing_res.render();
                let sending_res = send_mails(
                    &html, 
                    settings.mails.iter().cloned(), 
                    &settings.smtp
                );

                if let Err(mut sending_err) = sending_res {
                    sending_err.push_str("\nSaved in file");
                    let date = chrono::Local::now().naive_local();
                    let path = format!(
                        "./{}{:02}{:02}{:02}{:02}{:02}.html",
                        date.year(), date.month(), date.day(), 
                        date.hour(), date.minute(), date.second()
                    );
                    let _ = std::fs::write(path, html);
                }
                parsing_res.save_report();
            }
        }
    }
}


fn main() {
    panic_hook();
    init_main_loop();
}
