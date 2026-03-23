use serde::Deserialize;
use lettre::message::Mailbox;
use chrono::NaiveTime;


#[derive(Debug, Deserialize)]
pub struct Settings {
    pub time_table: Box<[NaiveTime]>,
    pub keywords: Box<[Box<str>]>,
    pub mails: Box<[Mailbox]>, 
    pub show_info: bool,
    pub pages_limit: u16,
    pub smtp: SMTPConfig,
}
impl Settings {
    pub fn load() -> Result<Self, String> {
        match std::fs::read_to_string("./settings.json") {
            Ok(s) =>  serde_json::from_str::<Settings>(&s).map_err(|e| e.to_string()),
            Err(e) => {
                Self::create_settings_file();
                Err(e.to_string())
            },
        }
    }
    fn create_settings_file() {
        const CONTENT: &str = concat!(
            "{\n",  
            r#"  "time_table": [],"#, "\n", 
            r#"  "keywords": [],"#, "\n", 
            r#"  "mails": [],"#, "\n", 
            r#"  "show_info": true,"#, "\n",
            r#"  "pages_limit": 65535,"#, "\n",
            r#"  "smtp": {"#, "\n", 
            r#"    "username": "","#, "\n",
            r#"    "password": "","#, "\n",
            r#"    "relay": """#, "\n",
            "  }\n}"
        );
        let _ = std::fs::write("./settings.json", CONTENT);
    }
}
#[derive(Debug, Deserialize)]
pub struct SMTPConfig {
    pub username: Mailbox,
    pub password: Box<str>,
    pub relay: Box<str>,
}